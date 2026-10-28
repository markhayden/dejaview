//! The durable evaluation ledger (SPEC "Runner and arms"): one append-only `ledger.jsonl` under
//! the evaluation directory. Every attempt at a decision has its own hold: the reservation is
//! written and fsynced before anything is sent, and its settlement is written once. On open, the
//! file is replayed: settled attempts count their settled cost, and an attempt reserved but never
//! settled, which a crash may have sent, is charged at its reservation (its bound). A decision run
//! again after a crash is a new attempt, charged on top of the one the crash left. Admission of
//! the next decision is against the cap less everything spent and reserved so far, across every
//! restart.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use dejaview_core::ledger::Ledger;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const LEDGER_FILE: &str = "ledger.jsonl";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum Line {
    Reserve {
        id: String,
        #[serde(default)]
        attempt: u32,
        usd: f64,
    },
    Settle {
        id: String,
        #[serde(default)]
        attempt: u32,
        reserved: f64,
        spent: f64,
    },
}

/// Where a crash is simulated, for the recovery tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    BeforeReservePersisted,
    AfterReservePersisted,
    BeforeSettlePersisted,
    AfterSettlePersisted,
}

fn key(id: &str, attempt: u32) -> String {
    format!("{id}#{attempt}")
}

#[derive(Debug, Default)]
struct State {
    /// Attempts reserved in this process and not yet settled: key -> reserved.
    open: BTreeMap<String, f64>,
    /// Attempts settled, by key.
    settled: BTreeMap<String, f64>,
    /// How many attempts each decision has had a reservation line for.
    attempts: BTreeMap<String, u32>,
    spent: f64,
    /// The decision the next reserve and settle belong to, and its attempt number.
    current: Option<(String, u32)>,
    fault: Option<Fault>,
}

/// The ledger: a cap, the file, and the replayed state.
pub struct EvalLedger {
    cap: f64,
    path: PathBuf,
    recovered: Vec<String>,
    state: Mutex<State>,
}

impl EvalLedger {
    /// Opens (creating) the ledger under `dir` and replays it.
    pub fn open(dir: &Path, cap: f64) -> Result<EvalLedger> {
        let path = dir.join(LEDGER_FILE);
        let mut reserved: BTreeMap<String, f64> = BTreeMap::new();
        let mut settled: BTreeMap<String, f64> = BTreeMap::new();
        let mut attempts: BTreeMap<String, u32> = BTreeMap::new();
        let mut spent = 0.0;
        if path.exists() {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| Error::Ledger(format!("cannot read {}: {e}", path.display())))?;
            for (n, raw) in text.lines().enumerate() {
                if raw.trim().is_empty() {
                    continue;
                }
                let line: Line = serde_json::from_str(raw).map_err(|e| {
                    Error::Ledger(format!("{} line {}: {e}", path.display(), n + 1))
                })?;
                match line {
                    Line::Reserve { id, attempt, usd } => {
                        let k = key(&id, attempt);
                        let seen = attempts.entry(id).or_insert(0);
                        *seen = (*seen).max(attempt);
                        reserved.entry(k).or_insert(usd);
                    }
                    Line::Settle {
                        id,
                        attempt,
                        reserved: _,
                        spent: s,
                    } => {
                        let k = key(&id, attempt);
                        if !settled.contains_key(&k) {
                            settled.insert(k.clone(), s);
                            spent += s;
                            reserved.remove(&k);
                        }
                    }
                }
            }
        }
        // Reserved and never settled: sent or possibly sent before a crash, charged at bound.
        let mut recovered = Vec::new();
        for (k, usd) in reserved {
            spent += usd;
            recovered.push(k.split_once('#').map(|(id, _)| id.to_string()).unwrap_or(k));
        }
        recovered.dedup();
        Ok(EvalLedger {
            cap,
            path,
            recovered,
            state: Mutex::new(State {
                settled,
                attempts,
                spent,
                ..State::default()
            }),
        })
    }

    /// Names the decision the next reserve and settle belong to: the attempt still open in this
    /// process if there is one, otherwise a new attempt of it.
    pub fn begin(&self, decision_id: &str) {
        if let Ok(mut s) = self.state.lock() {
            let open = s
                .open
                .keys()
                .filter_map(|k| k.split_once('#'))
                .find(|(id, _)| *id == decision_id)
                .and_then(|(_, n)| n.parse::<u32>().ok());
            let attempt =
                open.unwrap_or_else(|| s.attempts.get(decision_id).copied().unwrap_or(0) + 1);
            s.current = Some((decision_id.to_string(), attempt));
        }
    }

    /// Decisions with an attempt charged at its bound on this open because a crash left it
    /// unsettled.
    pub fn recovered(&self) -> &[String] {
        &self.recovered
    }

    /// Every decision with a settled attempt, in id order.
    pub fn settled(&self) -> Vec<String> {
        self.state
            .lock()
            .map(|s| {
                let mut ids: Vec<String> = s
                    .settled
                    .keys()
                    .filter_map(|k| k.split_once('#').map(|(id, _)| id.to_string()))
                    .collect();
                ids.dedup();
                ids
            })
            .unwrap_or_default()
    }

    /// Simulates a crash at the next fault point (tests).
    #[doc(hidden)]
    pub fn inject(&self, fault: Option<Fault>) {
        if let Ok(mut s) = self.state.lock() {
            s.fault = fault;
        }
    }

    fn append(&self, line: &Line) -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let mut text = serde_json::to_string(line).map_err(std::io::Error::other)?;
        text.push('\n');
        file.write_all(text.as_bytes())?;
        file.sync_data()
    }
}

fn budget(msg: String) -> dejaview_core::Error {
    dejaview_core::Error::Budget(msg)
}

impl Ledger for EvalLedger {
    fn cap(&self) -> f64 {
        self.cap
    }

    fn reserve(&self, usd: f64) -> dejaview_core::Result<()> {
        let mut s = self
            .state
            .lock()
            .map_err(|_| budget("evaluation ledger poisoned".into()))?;
        let (id, attempt) = s.current.clone().ok_or_else(|| {
            budget("evaluation ledger: reserve without a decision; call begin first".into())
        })?;
        let held: f64 = s.open.values().sum();
        let remaining = self.cap - s.spent - held;
        if usd > remaining + 1e-9 {
            return Err(budget(format!(
                "decision {id}: envelope ${usd:.4} exceeds the evaluation's remaining ${remaining:.4} of the ${:.2} cap",
                self.cap
            )));
        }
        if s.fault == Some(Fault::BeforeReservePersisted) {
            s.fault = None;
            return Err(budget(format!(
                "decision {id}: crashed before the reservation was persisted"
            )));
        }
        self.append(&Line::Reserve {
            id: id.clone(),
            attempt,
            usd,
        })
        .map_err(|e| {
            budget(format!(
                "evaluation ledger: cannot persist the reservation: {e}"
            ))
        })?;
        s.attempts.insert(id.clone(), attempt);
        if s.fault == Some(Fault::AfterReservePersisted) {
            s.fault = None;
            return Err(budget(format!(
                "decision {id}: crashed after the reservation was persisted"
            )));
        }
        s.open.insert(key(&id, attempt), usd);
        Ok(())
    }

    fn settle(&self, reserved_usd: f64, spent_usd: f64) {
        let Ok(mut s) = self.state.lock() else { return };
        let Some((id, attempt)) = s.current.clone() else {
            return;
        };
        let k = key(&id, attempt);
        if !s.open.contains_key(&k) {
            // At most once: only an attempt reserved in this process and still open settles. A
            // late settlement of one already settled, or charged at bound by a recovery, or never
            // reserved here, changes nothing.
            return;
        }
        if s.fault == Some(Fault::BeforeSettlePersisted) {
            s.fault = None;
            s.open.remove(&k);
            return;
        }
        if self
            .append(&Line::Settle {
                id,
                attempt,
                reserved: reserved_usd,
                spent: spent_usd,
            })
            .is_err()
        {
            return;
        }
        s.fault = None;
        s.open.remove(&k);
        s.settled.insert(k, spent_usd);
        s.spent += spent_usd;
    }

    fn reserved(&self) -> f64 {
        self.state
            .lock()
            .map(|s| s.open.values().sum())
            .unwrap_or(0.0)
    }

    fn spent(&self) -> f64 {
        self.state.lock().map(|s| s.spent).unwrap_or(0.0)
    }
}
