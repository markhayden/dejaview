//! A tool pack: one server process serving one run at a time under an exclusive lease. A run
//! acquires the lease before its first call, within `startup_timeout_s`, which covers waiting for
//! the previous lease's calls to finish, resetting the server (`dejaview/reset`) or, when the
//! server lacks reset, is tainted by a timed-out call, or fails to reset in time, shutting it down
//! and starting a fresh one whose manifests must equal the ones frozen at connect. Within a lease,
//! tools that declare `concurrent = true` run in parallel on the one process; the others take
//! turns.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use dejaview_core::tool::{DejaviewContext, Envelope, Manifest, Tool, inject_context};
use dejaview_core::{Error, Result};

use super::{Client, Reset, Spawn, Timeouts, envelope_from};

/// A connected pack.
pub struct Pack {
    name: String,
    spawn: Spawn,
    timeouts: Timeouts,
    environment: Vec<(String, String)>,
    frozen: Vec<Manifest>,
    state: Mutex<State>,
    /// Signalled when a call completes, for a lease waiting on the previous run's calls.
    settled: Condvar,
    /// Held for the whole of a call by a tool that is not `concurrent`.
    serial: Mutex<()>,
}

struct State {
    client: Option<Arc<Client>>,
    /// The run holding the lease.
    run: Option<String>,
    /// A server that has served no run yet needs no reset.
    fresh: bool,
    restarts: u32,
    /// Calls of the lease holder still running.
    in_flight: usize,
}

impl Pack {
    /// Starts the server once to freeze its manifests.
    pub fn connect<I, K, V>(
        name: &str,
        spawn: Spawn,
        timeouts: Timeouts,
        environment: I,
    ) -> Result<Arc<Pack>>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        // Only the allowlisted variables are kept for restarts; credentials never linger here.
        let environment: Vec<(String, String)> =
            dejaview_core::env::allowlist(&spawn.declared_env, environment)
                .into_iter()
                .collect();
        let client = Client::connect(
            &spawn,
            environment.iter().cloned(),
            Instant::now() + timeouts.startup,
        )?;
        let frozen = client.manifests();
        if frozen.is_empty() {
            client.shutdown(timeouts.shutdown_grace);
            return Err(Error::Config(format!("mcp: pack {name:?} lists no tools")));
        }
        Ok(Arc::new(Pack {
            name: name.to_string(),
            spawn,
            timeouts,
            environment,
            frozen,
            state: Mutex::new(State {
                client: Some(Arc::new(client)),
                run: None,
                fresh: true,
                restarts: 0,
                in_flight: 0,
            }),
            settled: Condvar::new(),
            serial: Mutex::new(()),
        }))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The manifests as frozen at connect.
    pub fn manifests(&self) -> &[Manifest] {
        &self.frozen
    }

    /// One `Tool` per frozen manifest, all sharing this pack.
    pub fn tools(self: &Arc<Self>) -> Vec<Box<dyn Tool>> {
        self.frozen
            .iter()
            .map(|m| {
                Box::new(PackTool {
                    pack: Arc::clone(self),
                    manifest: m.clone(),
                }) as Box<dyn Tool>
            })
            .collect()
    }

    /// Times the server was replaced.
    pub fn restarts(&self) -> u32 {
        self.state.lock().map(|s| s.restarts).unwrap_or(0)
    }

    /// The current server's pid, if one is running.
    pub fn pid(&self) -> Option<u32> {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.client.as_ref().map(|c| c.pid()))
    }

    /// The current server's stderr so far.
    pub fn stderr_lines(&self) -> Vec<String> {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.client.as_ref().map(|c| c.stderr_lines()))
            .unwrap_or_default()
    }

    /// Calls a tool for a run, acquiring the lease first. A tool that is not `concurrent` holds
    /// the pack's serial turn for the whole call.
    pub fn call(
        &self,
        run_id: &str,
        run_dir: Option<&std::path::Path>,
        tool: &str,
        arguments: serde_json::Value,
    ) -> Result<Envelope> {
        let manifest = self.frozen.iter().find(|m| m.name == tool).ok_or_else(|| {
            Error::Tool(format!("mcp: pack {:?} has no tool {tool:?}", self.name))
        })?;
        let client = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| Error::Tool("mcp: pack state poisoned".into()))?;
            let deadline = Instant::now() + self.timeouts.startup;
            state = self.acquire(state, run_id, run_dir, deadline)?;
            state.in_flight += 1;
            Arc::clone(
                state
                    .client
                    .as_ref()
                    .ok_or_else(|| Error::Tool("mcp: no server after acquisition".into()))?,
            )
        };
        let result = if manifest.concurrent {
            client.call(tool, arguments, self.timeouts.call)
        } else {
            let _turn = self
                .serial
                .lock()
                .map_err(|_| Error::Tool("mcp: pack turn poisoned".into()))?;
            client.call(tool, arguments, self.timeouts.call)
        };
        if let Ok(mut state) = self.state.lock() {
            state.in_flight = state.in_flight.saturating_sub(1);
            self.settled.notify_all();
        }
        envelope_from(result?, manifest)
    }

    /// The lease: the holder keeps it; a new run waits for the holder's calls to finish, then
    /// resets or restarts the server, all before one deadline. A tainted server is never reused.
    fn acquire<'a>(
        &'a self,
        mut state: std::sync::MutexGuard<'a, State>,
        run_id: &str,
        run_dir: Option<&std::path::Path>,
        deadline: Instant,
    ) -> Result<std::sync::MutexGuard<'a, State>> {
        let healthy = |s: &State| s.client.as_ref().is_some_and(|c| !c.is_tainted());
        if state.run.as_deref() == Some(run_id) && healthy(&state) {
            return Ok(state);
        }
        // Another run's calls still running: wait for them, within the one deadline.
        while state.in_flight > 0 {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Error::Tool(format!(
                    "mcp: pack {:?}: lease acquisition passed its {:?} deadline waiting for the previous run's calls",
                    self.name, self.timeouts.startup
                )));
            }
            let (guard, _) = self
                .settled
                .wait_timeout(state, remaining)
                .map_err(|_| Error::Tool("mcp: pack state poisoned".into()))?;
            state = guard;
        }
        let reusable = healthy(&state);
        if reusable && state.fresh {
            state.fresh = false;
            state.run = Some(run_id.to_string());
            return Ok(state);
        }
        if reusable && let Some(client) = state.client.as_ref() {
            match client.reset(deadline) {
                Ok(Reset::Acknowledged) => {
                    state.run = Some(run_id.to_string());
                    return Ok(state);
                }
                Ok(Reset::Unsupported) => {}
                Err(_) => {} // hung or failed: fall through to a restart under the same deadline
            }
        }
        if let Some(old) = state.client.take() {
            old.shutdown(self.timeouts.shutdown_grace);
        }
        if Instant::now() >= deadline {
            return Err(Error::Tool(format!(
                "mcp: pack {:?}: lease acquisition passed its {:?} deadline before a restart",
                self.name, self.timeouts.startup
            )));
        }
        let mut spawn = self.spawn.clone();
        if let Some(dir) = run_dir {
            spawn.cwd = Some(dir.to_path_buf());
        }
        let client = Client::connect(&spawn, self.environment.iter().cloned(), deadline)?;
        if client.manifests() != self.frozen {
            client.shutdown(self.timeouts.shutdown_grace);
            return Err(Error::Tool(format!(
                "mcp: pack {:?}: the restarted server lists different tools",
                self.name
            )));
        }
        state.client = Some(Arc::new(client));
        state.run = Some(run_id.to_string());
        state.fresh = false;
        state.restarts += 1;
        Ok(state)
    }

    /// Shuts the server down.
    pub fn shutdown(&self) {
        if let Ok(mut state) = self.state.lock()
            && let Some(client) = state.client.take()
        {
            client.shutdown(self.timeouts.shutdown_grace);
        }
    }
}

impl Drop for Pack {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// One tool of a pack, behind the `Tool` trait.
pub struct PackTool {
    pack: Arc<Pack>,
    manifest: Manifest,
}

impl Tool for PackTool {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn call(&self, args: &serde_json::Value, ctx: &DejaviewContext) -> Result<Envelope> {
        let full = inject_context(args, ctx)?;
        self.pack.call(
            &ctx.run_id,
            ctx.run_dir.as_deref(),
            &self.manifest.name,
            full,
        )
    }
}
