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
    pub prompt_version: String,
    pub provider: String,
    /// Enabled tools as `name@version`, in prompt order.
    pub tools: Vec<String>,
    /// How many runs the decision asked for, and the runs themselves, in order.
    pub decision_runs: u32,
    pub runs: Vec<RunReport>,
    pub decision: Decision,
    /// The combined verdict, `None` when the decision is an error.
    pub verdict: Option<Verdict>,
    pub policy: PolicyConfig,
    pub exit_code: i32,
    /// The decision envelope and what it came to.
    pub cost: CostSummary,
}

/// Money for the decision (SPEC "Cost ledger").
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostSummary {
    /// The planned envelope, `None` when the rules did not resolve.
    pub envelope: Option<crate::ledger::Envelope>,
    pub cap_usd: f64,
    pub spent_usd: f64,
    pub released_usd: f64,
    pub overrun_usd: f64,
    /// Billable attempts made.
    pub attempts: u32,
}

/// One run's report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReport {
    pub run: u32,
    pub images: Vec<CanonicalImage>,
    pub instances: Vec<InstanceEntry>,
    pub checklist: Checklist,
    pub turns: Vec<TurnEntry>,
    pub tool_calls: Vec<ToolCallEntry>,
    /// Repairs taken, in order (at most `budget.repairs`).
    pub repairs: Vec<RepairEntry>,
    pub items: Vec<ItemRow>,
    pub verdict: Option<Verdict>,
    pub error: Option<ErrorEntry>,
}

/// One model turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnEntry {
    pub turn: u32,
    pub stop_reason: crate::provider::StopReason,
    pub tool_calls: Vec<String>,
    pub usage: crate::provider::Usage,
    pub provider_request_id: Option<String>,
    /// The output allowance of the request (raised to `repair_output_tokens` on a reissue).
    pub max_output_tokens: u32,
    /// Billable attempts the request took (1 plus transport retries).
    pub attempts: u32,
    /// The bound of one attempt at catalog prices, and what the turn came to (settled usage plus
    /// every failed attempt charged at its bound).
    pub bound_usd: f64,
    pub cost_usd: f64,
    /// Whether the response entered provider history (false for an excluded turn).
    pub entered_history: bool,
}

/// One repair (SPEC "Repair protocol").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepairEntry {
    /// `rejected` (the response stayed in history, error results answered it) or `reissue` (the
    /// response was excluded and the request sent again).
    pub form: String,
    /// `invalid_submission`, `mixed_batch`, `stray_call`, `truncated`, `refusal` or `no_call`.
    pub reason: String,
    /// The turn number whose response was repaired.
    pub after_turn: u32,
}

/// One tool call as the harness handled it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEntry {
    pub id: String,
    pub provider_call_id: String,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub status: Option<crate::tool::Status>,
    pub invalid_args: Option<String>,
    pub duration_ms: u64,
    pub added_items: Vec<String>,
    pub dropped: Vec<String>,
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
