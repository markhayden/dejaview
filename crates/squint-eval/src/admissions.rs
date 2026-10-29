//! `evalsets/admissions.toml`: which tools are admitted for which use case, at which gate stage,
//! by which receipt. Written by `squint eval gate` (3.5b); read by the runner, which lets arm D
//! include only tools whose stage is `confirmed`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const ADMISSIONS_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Admissions {
    pub admissions_version: u32,
    #[serde(default)]
    pub admission: Vec<Admission>,
}

impl Default for Admissions {
    fn default() -> Admissions {
        Admissions {
            admissions_version: ADMISSIONS_VERSION,
            admission: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Admission {
    pub tool: String,
    pub use_case: String,
    pub stage: Stage,
    /// The id of the receipt `eval gate` wrote for the stage that produced this record.
    pub receipt: String,
}

/// `provisional` after an admitted initial stage; `confirmed` after an admitted confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Provisional,
    Confirmed,
}

impl Admissions {
    pub fn parse(text: &str) -> Result<Admissions> {
        let a: Admissions = toml::from_str(text).map_err(|e| Error::Admissions(e.to_string()))?;
        if a.admissions_version != ADMISSIONS_VERSION {
            return Err(Error::Admissions(format!(
                "admissions_version must be {ADMISSIONS_VERSION}, got {}",
                a.admissions_version
            )));
        }
        let mut seen = std::collections::BTreeSet::new();
        for r in &a.admission {
            if !seen.insert((r.tool.as_str(), r.use_case.as_str())) {
                return Err(Error::Admissions(format!(
                    "tool {:?} for {:?} is recorded twice",
                    r.tool, r.use_case
                )));
            }
        }
        Ok(a)
    }

    pub fn load(path: &Path) -> Result<Admissions> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Admissions(format!("cannot read {}: {e}", path.display())))?;
        Admissions::parse(&text).map_err(|e| Error::Admissions(format!("{}: {e}", path.display())))
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).map_err(|e| Error::Admissions(e.to_string()))?;
        crate::write_atomic(path, text.as_bytes())
            .map_err(|e| Error::Admissions(format!("cannot write {}: {e}", path.display())))
    }

    /// The tools arm D includes for a use case: those confirmed, sorted.
    pub fn admitted(&self, use_case: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .admission
            .iter()
            .filter(|r| r.use_case == use_case && r.stage == Stage::Confirmed)
            .map(|r| r.tool.clone())
            .collect();
        out.sort();
        out
    }

    pub fn stage(&self, tool: &str, use_case: &str) -> Option<Stage> {
        self.admission
            .iter()
            .find(|r| r.tool == tool && r.use_case == use_case)
            .map(|r| r.stage)
    }
}
