//! The MCP client (SPEC "Transports", MCP over stdio, revision 2025-11-25) in its minimal form:
//! the lifecycle contract against a server on stdio. Pagination beyond a cursor loop, concurrent
//! calls and the `rmcp` decision are tasks 4.4a and 4.4b.
//!
//! The client is synchronous, like the `Tool` trait it serves: a reader thread turns stdout lines
//! into messages on a channel, stderr is captured, and every wait has a deadline. A call that
//! passes its deadline is cancelled with `notifications/cancelled`, its id tombstoned so a late
//! answer is ignored, and the process marked unreusable.

pub mod pack;

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dejaview_core::tool::{Envelope, Manifest, Status};
use dejaview_core::{Error, Result};
use serde_json::{Value, json};

pub use pack::{Pack, PackTool};

/// The one revision the first release speaks.
pub const PROTOCOL_VERSION: &str = "2025-11-25";

/// How to start a server.
#[derive(Debug, Clone)]
pub struct Spawn {
    pub command: PathBuf,
    pub args: Vec<String>,
    /// Environment variable names the pack declares; filtered by `dejaview_core::env::allowlist`.
    pub declared_env: Vec<String>,
    /// The working directory (the run directory); the current directory when `None`.
    pub cwd: Option<PathBuf>,
    /// Harness-owned variables set after the allowlist (`UV_OFFLINE`, the lock digest).
    pub extra_env: Vec<(String, String)>,
}

/// The clocks of the contract.
#[derive(Debug, Clone, Copy)]
pub struct Timeouts {
    /// One bounded deadline for acquiring a lease: startup, reset or restart.
    pub startup: Duration,
    /// Per-call deadline (`budget.tool_timeout_s`).
    pub call: Duration,
    /// Each step of shutdown escalation waits this long: stdin closed, SIGTERM, SIGKILL.
    pub shutdown_grace: Duration,
}

/// A tool as the server listed it, with its manifest assembled from the native fields and the
/// `dejaview` annotation.
#[derive(Debug, Clone)]
pub struct ListedTool {
    pub name: String,
    pub manifest: Manifest,
}

/// What `tools/call` returned.
#[derive(Debug, Clone)]
pub struct CallResult {
    pub structured: Value,
    pub content: Vec<Value>,
    pub is_error: bool,
}

/// Whether the server honoured `dejaview/reset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reset {
    Acknowledged,
    Unsupported,
}

enum Incoming {
    Message(Value),
    Eof,
}

/// One server process, initialised and listed.
pub struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<Incoming>,
    stderr: Arc<Mutex<Vec<String>>>,
    next_id: u64,
    tombstones: BTreeSet<u64>,
    tainted: bool,
    tools: Vec<ListedTool>,
    server_info: Value,
}

fn tool_err(message: impl Into<String>) -> Error {
    Error::Tool(format!("mcp: {}", message.into()))
}

fn resolve_err(message: impl Into<String>) -> Error {
    Error::Config(format!("mcp: {}", message.into()))
}

impl Client {
    /// Spawns the server with the allowlisted environment, initialises it, and lists its tools,
    /// all before `deadline`. A wrong protocol version or a missing `tools` capability is
    /// `error: resolve`.
    pub fn connect<I, K, V>(spawn: &Spawn, environment: I, deadline: Instant) -> Result<Client>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut env = dejaview_core::env::allowlist(&spawn.declared_env, environment);
        env.extend(spawn.extra_env.iter().cloned());
        let mut command = Command::new(&spawn.command);
        command
            .args(&spawn.args)
            .env_clear()
            .envs(&env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = &spawn.cwd {
            command.current_dir(cwd);
        }
        let mut child = command
            .spawn()
            .map_err(|e| resolve_err(format!("cannot start {}: {e}", spawn.command.display())))?;
        let stdin = child.stdin.take().ok_or_else(|| tool_err("no stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| tool_err("no stdout"))?;
        let stderr_pipe = child.stderr.take().ok_or_else(|| tool_err("no stderr"))?;

        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) if l.trim().is_empty() => continue,
                    Ok(l) => match serde_json::from_str::<Value>(&l) {
                        Ok(v) => {
                            if tx.send(Incoming::Message(v)).is_err() {
                                return;
                            }
                        }
                        Err(_) => {
                            // A malformed frame ends the conversation: the reader stops and the
                            // next wait sees end of stream.
                            let _ = tx.send(Incoming::Eof);
                            return;
                        }
                    },
                    Err(_) => break,
                }
            }
            let _ = tx.send(Incoming::Eof);
        });
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&stderr);
        std::thread::spawn(move || {
            for line in BufReader::new(stderr_pipe).lines().map_while(|l| l.ok()) {
                if let Ok(mut v) = sink.lock() {
                    v.push(line);
                }
            }
        });

        let mut client = Client {
            child,
            stdin: Some(stdin),
            rx,
            stderr,
            next_id: 1,
            tombstones: BTreeSet::new(),
            tainted: false,
            tools: Vec::new(),
            server_info: Value::Null,
        };
        if let Err(e) = client.initialize(deadline) {
            client.shutdown(Duration::from_millis(200));
            return Err(e);
        }
        Ok(client)
    }

    fn initialize(&mut self, deadline: Instant) -> Result<()> {
        let result = self.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "dejaview", "version": env!("CARGO_PKG_VERSION") },
            }),
            deadline,
        )?;
        let version = result["protocolVersion"].as_str().unwrap_or("");
        if version != PROTOCOL_VERSION {
            return Err(resolve_err(format!(
                "the server speaks protocol {version:?}; this release requires {PROTOCOL_VERSION:?}"
            )));
        }
        if !result["capabilities"]["tools"].is_object() {
            return Err(resolve_err(
                "the server does not advertise the `tools` capability",
            ));
        }
        self.server_info = result["serverInfo"].clone();
        self.notify("notifications/initialized", json!({}))?;
        self.list_tools(deadline)
    }

    fn list_tools(&mut self, deadline: Instant) -> Result<()> {
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let params = match &cursor {
                Some(c) => json!({ "cursor": c }),
                None => json!({}),
            };
            let page = self.request("tools/list", params, deadline)?;
            for t in page["tools"].as_array().cloned().unwrap_or_default() {
                tools.push(listed_tool(&t)?);
            }
            match page["nextCursor"].as_str() {
                Some(c) if !c.is_empty() => cursor = Some(c.to_string()),
                _ => break,
            }
        }
        self.tools = tools;
        Ok(())
    }

    pub fn tools(&self) -> &[ListedTool] {
        &self.tools
    }

    pub fn manifests(&self) -> Vec<Manifest> {
        self.tools.iter().map(|t| t.manifest.clone()).collect()
    }

    /// Whether a call in this process timed out or was cancelled; such a process is never reused.
    pub fn is_tainted(&self) -> bool {
        self.tainted
    }

    /// The server's stderr so far (for the journal).
    pub fn stderr_lines(&self) -> Vec<String> {
        self.stderr.lock().map(|v| v.clone()).unwrap_or_default()
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    fn send(&mut self, message: &Value) -> Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| tool_err("the server's stdin is closed"))?;
        let mut text = serde_json::to_string(message)?;
        text.push('\n');
        stdin
            .write_all(text.as_bytes())
            .and_then(|()| stdin.flush())
            .map_err(|e| tool_err(format!("cannot write to the server: {e}")))
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    /// Sends a request and waits for its response until `deadline`, skipping tombstoned ids and
    /// notifications. A timeout cancels the request and taints the process.
    fn request(&mut self, method: &str, params: Value, deadline: Instant) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(self.cancel(id, method));
            }
            match self.rx.recv_timeout(remaining) {
                Ok(Incoming::Message(m)) => {
                    let Some(mid) = m.get("id").and_then(Value::as_u64) else {
                        continue; // a notification or a server request: not ours to answer here
                    };
                    if self.tombstones.remove(&mid) {
                        continue; // a late answer to a cancelled call
                    }
                    if mid != id {
                        return Err(tool_err(format!(
                            "response for unknown id {mid} while waiting for {id} ({method})"
                        )));
                    }
                    if let Some(err) = m.get("error") {
                        return Err(Error::Tool(format!(
                            "mcp: {method} failed: {} (code {})",
                            err["message"].as_str().unwrap_or("?"),
                            err["code"]
                        )));
                    }
                    return Ok(m.get("result").cloned().unwrap_or(Value::Null));
                }
                Ok(Incoming::Eof) => {
                    return Err(tool_err(format!(
                        "the server ended the conversation during {method}"
                    )));
                }
                Err(RecvTimeoutError::Timeout) => return Err(self.cancel(id, method)),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(tool_err(format!("the server died during {method}")));
                }
            }
        }
    }

    fn cancel(&mut self, id: u64, method: &str) -> Error {
        self.tombstones.insert(id);
        self.tainted = true;
        let _ = self.notify(
            "notifications/cancelled",
            json!({ "requestId": id, "reason": "deadline" }),
        );
        tool_err(format!("{method} (id {id}) timed out; cancelled"))
    }

    /// `tools/call` with the full arguments (model arguments plus `_dejaview`).
    pub fn call(&mut self, name: &str, arguments: Value, timeout: Duration) -> Result<CallResult> {
        let deadline = Instant::now() + timeout;
        let result = self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
            deadline,
        )?;
        Ok(CallResult {
            structured: result
                .get("structuredContent")
                .cloned()
                .unwrap_or(Value::Null),
            content: result["content"].as_array().cloned().unwrap_or_default(),
            is_error: result["isError"].as_bool().unwrap_or(false),
        })
    }

    /// `dejaview/reset`: the server drops every prior-run task and mutable state before answering.
    pub fn reset(&mut self, deadline: Instant) -> Result<Reset> {
        match self.request("dejaview/reset", json!({}), deadline) {
            Ok(_) => Ok(Reset::Acknowledged),
            Err(Error::Tool(m)) if m.contains("(code -32601)") => Ok(Reset::Unsupported),
            Err(e) => Err(e),
        }
    }

    /// Close stdin, wait, SIGTERM, wait, SIGKILL.
    pub fn shutdown(mut self, grace: Duration) {
        drop(self.stdin.take());
        if wait_for_exit(&mut self.child, grace) {
            return;
        }
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-TERM", &self.child.id().to_string()])
                .status();
            if wait_for_exit(&mut self.child, grace) {
                return;
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn wait_for_exit(child: &mut Child, grace: Duration) -> bool {
    let deadline = Instant::now() + grace;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => return false,
        }
    }
}

/// A listed MCP tool to a dejaview manifest: `name`, `inputSchema` and `outputSchema` from the
/// native fields, the rest from the `dejaview` annotation; `description` must equal `purpose`.
fn listed_tool(tool: &Value) -> Result<ListedTool> {
    let name = tool["name"]
        .as_str()
        .ok_or_else(|| resolve_err("a listed tool has no name"))?
        .to_string();
    let mut manifest = tool["annotations"]["dejaview"]
        .as_object()
        .cloned()
        .ok_or_else(|| {
            resolve_err(format!(
                "tool {name:?} carries no `dejaview` manifest in its annotations"
            ))
        })?;
    manifest.insert("name".into(), Value::String(name.clone()));
    if let Some(schema) = tool.get("inputSchema") {
        manifest.insert("input_schema".into(), schema.clone());
    }
    if let Some(schema) = tool.get("outputSchema") {
        manifest.insert("output_schema".into(), schema.clone());
    }
    let manifest = Manifest::from_json(&Value::Object(manifest))?;
    let description = tool["description"].as_str().unwrap_or("");
    if description != manifest.purpose {
        return Err(resolve_err(format!(
            "tool {name:?}: description must equal the manifest's purpose"
        )));
    }
    Ok(ListedTool { name, manifest })
}

/// A call result to an envelope: `structuredContent` is the envelope; image content blocks carry
/// the bytes of `images` in order and media type. `isError` with an `invalid_args` envelope is
/// the one recoverable form; any other error, or a block mismatch, is `error: tool`.
pub fn envelope_from(result: CallResult, tool: &str) -> Result<Envelope> {
    let mut envelope: Envelope = serde_json::from_value(result.structured).map_err(|e| {
        tool_err(format!(
            "{tool}: structuredContent is not a result envelope: {e}"
        ))
    })?;
    if result.is_error && envelope.status != Status::InvalidArgs {
        return Err(tool_err(format!(
            "{tool}: isError with status {:?}: {}",
            envelope.status, envelope.summary
        )));
    }
    let images: Vec<&Value> = result
        .content
        .iter()
        .filter(|b| b["type"] == "image")
        .collect();
    if images.len() != envelope.images.len() {
        return Err(tool_err(format!(
            "{tool}: {} image blocks for {} envelope images",
            images.len(),
            envelope.images.len()
        )));
    }
    for (block, image) in images.iter().zip(envelope.images.iter_mut()) {
        let mime = block["mimeType"].as_str().unwrap_or("");
        if mime != image.media_type {
            return Err(tool_err(format!(
                "{tool}: image block {mime:?} for envelope image {:?}",
                image.media_type
            )));
        }
        image.base64 = block["data"].as_str().map(str::to_string);
    }
    Ok(envelope)
}

/// Environment pairs the harness passes to a spawn: the process environment by default.
pub fn process_environment() -> BTreeMap<String, String> {
    std::env::vars().collect()
}

#[cfg(test)]
mod minimal;
