//! The report: derived from the journal at the end of a run. Task 1.1 writes the fields the
//! walking slice has; `schemas/report-v1.json`, rebuild from a partial journal and prune arrive in
//! task 1.10b.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::aggregate::{Confidence, Disposition, Row, Verdict};
use crate::checklist::Checklist;
use crate::image::CanonicalImage;
use crate::job::JobSummary;
use crate::obligations::Instance;
use crate::submission::Location;

pub const REPORT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub report_version: u32,
    pub harness_version: String,
    pub use_case: String,
    /// The job as resolved (base64 payloads summarised).
    pub job: JobSummary,
    /// The config as resolved, with every expanded secret redacted.
    pub config: serde_json::Value,
    /// The rules as merged (base, profile, versions, obligations with provenance, warnings).
    pub rules: Option<crate::run::RulesSummary>,
    pub images: Vec<CanonicalImage>,
    pub instances: Vec<InstanceEntry>,
    pub checklist: Checklist,
    pub items: Vec<ItemRow>,
    pub provider: String,
    /// `pass | fail | unsure`, absent when the run ended in an error.
    pub verdict: Option<Verdict>,
    /// The spec's error class when the run ended in an error.
    pub error: Option<ErrorEntry>,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceEntry {
    pub id: String,
    #[serde(flatten)]
    pub instance: Instance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemRow {
    pub id: String,
    pub disposition: Disposition,
    pub confidence: Confidence,
    pub reason: String,
    pub location: Option<Location>,
    pub row: Row,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub class: String,
    pub message: String,
}

impl Report {
    /// Writes `report.json` into the run directory.
    pub fn write(&self, run_dir: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(run_dir.join("report.json"), text)?;
        Ok(())
    }
}
