//! The durable evaluation ledger (SPEC "Runner and arms"): one append-only `ledger.jsonl` under
//! the evaluation directory. A decision's reservation is written and fsynced before anything is
//! sent; its settlement is written once. On open, the file is replayed: settled decisions count
//! their settled cost, and a decision reserved but never settled, which a crash may have sent,
//! is charged at its reservation (its bound). Admission of the next decision is against the cap
//! less everything spent and reserved so far, across every restart.

use std::collections::{BTreeMap, BTreeSet};
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
        usd: f64,
    },
    Settle {
        id: String,
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

#[derive(Debug, Default)]
struct State {
    /// Decisions reserved in this process and not yet settled: id -> reserved.
    open: BTreeMap<String, f64>,
    settled: BTreeSet<String>,
    /// Decisions charged at their bound on recovery; closed, like settled ones.
    charged: BTreeSet<String>,
    spent: f64,
    current: Option<String>,
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
        let mut settled = BTreeSet::new();
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
                    Line::Reserve { id, usd } => {
                        reserved.insert(id, usd);
                    }
                    Line::Settle {
                        id,
                        reserved: _,
                        spent: s,
                    } => {
                        if settled.insert(id.clone()) {
                            spent += s;
                            reserved.remove(&id);
                        }
                    }
                }
            }
        }
        // Reserved and never settled: sent or possibly sent before a crash, charged at bound.
        let mut recovered = Vec::new();
        let mut charged = BTreeSet::new();
        for (id, usd) in reserved {
            spent += usd;
            charged.insert(id.clone());
            recovered.push(id);
        }
        Ok(EvalLedger {
            cap,
            path,
            recovered,
            state: Mutex::new(State {
                settled,
                charged,
                spent,
                ..State::default()
            }),
        })
    }

    /// Names the decision the next reserve and settle belong to.
    pub fn begin(&self, decision_id: &str) {
        if let Ok(mut s) = self.state.lock() {
            s.current = Some(decision_id.to_string());
        }
    }

    /// Decisions charged at their bound on this open because a crash left them unsettled.
    pub fn recovered(&self) -> &[String] {
        &self.recovered
    }

    /// Every decision settled or charged, in id order.
    pub fn settled(&self) -> Vec<String> {
        self.state
            .lock()
            .map(|s| s.settled.iter().cloned().collect())
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
        let id = s.current.clone().ok_or_else(|| {
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
            usd,
        })
        .map_err(|e| {
            budget(format!(
                "evaluation ledger: cannot persist the reservation: {e}"
            ))
        })?;
        if s.fault == Some(Fault::AfterReservePersisted) {
            s.fault = None;
            return Err(budget(format!(
                "decision {id}: crashed after the reservation was persisted"
            )));
        }
        s.open.insert(id, usd);
        Ok(())
    }

    fn settle(&self, reserved_usd: f64, spent_usd: f64) {
        let Ok(mut s) = self.state.lock() else { return };
        let Some(id) = s.current.clone() else { return };
        if s.settled.contains(&id) || s.charged.contains(&id) {
            // At most once: a late settlement of a decision already settled or charged.
            s.open.remove(&id);
            return;
        }
        if s.fault == Some(Fault::BeforeSettlePersisted) {
            s.fault = None;
            s.open.remove(&id);
            return;
        }
        if self
            .append(&Line::Settle {
                id: id.clone(),
                reserved: reserved_usd,
                spent: spent_usd,
            })
            .is_err()
        {
            return;
        }
        s.fault = None;
        s.open.remove(&id);
        s.settled.insert(id);
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
