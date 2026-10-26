//! The report (SPEC "Report and journal"): one JSON document per decision, holding every run in
//! full. Derived from the runs' journals at the end; `schemas/report-v1.json`, rebuild from a
//! partial journal and prune arrive in task 1.10b.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::aggregate::{Confidence, Disposition, Row, Verdict};
use crate::checklist::Checklist;
use crate::config::PolicyConfig;
use crate::decision::Decision;
use crate::frames::Location;
use crate::image::CanonicalImage;
use crate::job::JobSummary;
use crate::obligations::Instance;
use crate::run::RulesSummary;

pub const REPORT_VERSION: u32 = 1;

/// The decision-level report.
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
    pub rules: Option<RulesSummary>,
    pub provider: String,
    /// How many runs the decision asked for, and the runs themselves, in order.
    pub decision_runs: u32,
    pub runs: Vec<RunReport>,
    pub decision: Decision,
    /// The combined verdict, `None` when the decision is an error.
    pub verdict: Option<Verdict>,
    pub policy: PolicyConfig,
    pub exit_code: i32,
}

/// One run's report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReport {
    pub run: u32,
    pub images: Vec<CanonicalImage>,
    pub instances: Vec<InstanceEntry>,
    pub checklist: Checklist,
    pub items: Vec<ItemRow>,
    pub verdict: Option<Verdict>,
    pub error: Option<ErrorEntry>,
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
    pub allowed_by: Option<String>,
    pub location: Option<Location>,
    pub row: Row,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub class: String,
    pub message: String,
}

impl Report {
    /// Writes `report.json` into the decision directory.
    pub fn write(&self, dir: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(dir.join("report.json"), text)?;
        Ok(())
    }
}
