//! The run: resolve the job, show it to the model, take the submission, aggregate, record. Task
//! 1.1 is the walking slice: one turn, one submission. Tool turns, the final turn, repairs, the
//! ledger and the cache arrive in tasks 1.7a to 1.11.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::aggregate::{self, Row, Verdict};
use crate::checklist::{self, ChecklistItem};
use crate::config::Config;
use crate::image::{self, CanonicalImage};
use crate::job::{Job, JobSummary};
use crate::journal::Journal;
use crate::obligations::{self, Instance, MergeInput, Resolved, RulesFile};
use crate::provider::{ContentBlock, Message, MessageRole, Provider, StopReason, TurnRequest};
use crate::render;
use crate::report::{ErrorEntry, InstanceEntry, ItemRow, REPORT_VERSION, Report};
use crate::submission;
use crate::{Error, Result};

/// What `check` produced: a verdict or an error, and where the run directory is.
#[derive(Debug)]
pub struct Outcome {
    pub run_dir: PathBuf,
    pub verdict: Option<Verdict>,
    pub error: Option<Error>,
    pub exit_code: i32,
}

/// Exit code for a run that ended in an error.
pub const EXIT_ERROR: i32 = 3;

/// Runs one check end to end and writes its journal and report under `out_dir`.
pub async fn check(
    config: &Config,
    job: &Job,
    job_base: &Path,
    provider: &dyn Provider,
    out_dir: &Path,
    harness_version: &str,
) -> Result<Outcome> {
    let run_dir = new_run_dir(out_dir)?;
    let mut journal = Journal::create(&run_dir)?;
    let mut state = RunState::new(config, job, provider, harness_version);

    let result = drive(config, job, job_base, provider, &mut journal, &mut state).await;
    let outcome = match result {
        Ok(verdict) => {
            journal.append("verdict", serde_json::json!({ "verdict": verdict }))?;
            Outcome {
                run_dir: run_dir.clone(),
                verdict: Some(verdict),
                error: None,
                exit_code: verdict.exit_code(),
            }
        }
        Err(e) => {
            journal.append(
                "error",
                serde_json::json!({ "class": e.class(), "message": e.to_string() }),
            )?;
            state.error = Some(ErrorEntry {
                class: e.class().into(),
                message: e.to_string(),
            });
            Outcome {
                run_dir: run_dir.clone(),
                verdict: None,
                error: Some(e),
                exit_code: EXIT_ERROR,
            }
        }
    };
    state
        .report(outcome.verdict, outcome.exit_code)
        .write(&run_dir)?;
    Ok(outcome)
}

struct RunState {
    use_case: String,
    job: JobSummary,
    config: serde_json::Value,
    rules: Option<RulesSummary>,
    resolved: Option<Resolved>,
    provider_id: String,
    harness_version: String,
    images: Vec<CanonicalImage>,
    instances: Vec<Instance>,
    checklist: Vec<ChecklistItem>,
    items: Vec<ItemRow>,
    error: Option<ErrorEntry>,
}

impl RunState {
    fn new(config: &Config, job: &Job, provider: &dyn Provider, harness_version: &str) -> RunState {
        RunState {
            use_case: job.use_case.clone(),
            job: job.summary(),
            config: config.redacted_json(),
            rules: None,
            resolved: None,
            provider_id: provider.id().to_string(),
            harness_version: harness_version.to_string(),
            images: Vec::new(),
            instances: Vec::new(),
            checklist: Vec::new(),
            items: Vec::new(),
            error: None,
        }
    }

    fn report(&self, verdict: Option<Verdict>, exit_code: i32) -> Report {
        Report {
            report_version: REPORT_VERSION,
            harness_version: self.harness_version.clone(),
            use_case: self.use_case.clone(),
            job: self.job.clone(),
            config: self.config.clone(),
            rules: self.rules.clone(),
            images: self.images.clone(),
            instances: self
                .instances
                .iter()
                .map(|i| InstanceEntry {
                    id: i.id(),
                    instance: i.clone(),
                })
                .collect(),
            checklist: self.checklist.clone(),
            items: self.items.clone(),
            provider: self.provider_id.clone(),
            verdict,
            error: self.error.clone(),
            exit_code,
        }
    }
}

async fn drive(
    config: &Config,
    job: &Job,
    job_base: &Path,
    provider: &dyn Provider,
    journal: &mut Journal,
    state: &mut RunState,
) -> Result<Verdict> {
    // Resolve.
    let bounds = config.images.decode_bounds();
    state.images = job
        .images
        .iter()
        .map(|spec| image::load(spec, job_base, &bounds))
        .collect::<Result<Vec<_>>>()?;
    let prepared = resolve_rules(config, job, job_base)?;
    state.rules = Some(prepared.summary.clone());
    state.instances = prepared.resolved.instances.clone();
    state.resolved = Some(prepared.resolved);
    state.checklist = checklist::build(&state.instances);
    journal.append(
        "resolved",
        serde_json::json!({
            "images": state.images.iter().map(|i| serde_json::json!({"id": i.id, "role": i.role, "sha256": i.sha256, "width": i.width, "height": i.height})).collect::<Vec<_>>(),
            "instances": state.instances.iter().map(Instance::id).collect::<Vec<_>>(),
            "checklist": state.checklist.iter().map(|c| c.id.clone()).collect::<Vec<_>>(),
        }),
    )?;

    // First look: every image, the checklist, the instruction to submit.
    let system = match &state.resolved {
        Some(r) => render::system_prompt(r, job),
        None => String::new(),
    };
    let mut content: Vec<ContentBlock> = state
        .images
        .iter()
        .map(|img| ContentBlock::Text {
            text: format!(
                "Image `{}` (role {:?}{}, frame raw:{}, {}x{}).",
                img.id,
                img.role,
                img.label
                    .as_deref()
                    .map(|l| format!(", {l}"))
                    .unwrap_or_default(),
                img.id,
                img.width,
                img.height
            ),
        })
        .flat_map(|label| {
            [
                label.clone(),
                ContentBlock::Image {
                    image_id: image_id_of(&label),
                },
            ]
        })
        .collect();
    content.push(ContentBlock::Text {
        text: checklist::render(&state.checklist, &state.instances),
    });
    content.push(ContentBlock::Text {
        text:
            "Inspect the images, then call submit_verdict with one disposition per checklist item."
                .into(),
    });
    let history = vec![Message {
        role: MessageRole::User,
        content,
    }];
    let tools = vec![submission::submit_verdict_tool()];
    let request = TurnRequest {
        system: &system,
        history: &history,
        tools: &tools,
        max_output_tokens: 16_000,
    };
    journal.append(
        "turn_sent",
        serde_json::json!({ "turn": 1, "tools": tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>(), "images": state.images.len() }),
    )?;
    let response = provider.send(request).await?;
    journal.append(
        "turn_received",
        serde_json::json!({
            "turn": 1,
            "stop_reason": response.stop_reason,
            "tool_calls": response.tool_calls.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
            "usage": response.usage,
        }),
    )?;

    // The walking slice accepts exactly one submit_verdict call and nothing else.
    let call = match response.tool_calls.as_slice() {
        [call] if call.name == "submit_verdict" => call,
        [] => {
            return Err(Error::NoSubmission(format!(
                "the model ended the turn ({:?}) without calling submit_verdict",
                response.stop_reason
            )));
        }
        calls => {
            return Err(Error::NoSubmission(format!(
                "expected a single submit_verdict call, got {:?}",
                calls.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()
            )));
        }
    };
    if response.stop_reason == StopReason::MaxTokens {
        return Err(Error::Provider(
            "the response was truncated (max_tokens); repairs arrive in task 1.8".into(),
        ));
    }
    let sub = submission::parse_and_validate(&call.arguments, &state.checklist)?;
    journal.append("submission", serde_json::json!({ "call_id": call.id, "items": sub.items.len(), "findings": sub.findings.len() }))?;

    // Aggregate: one row per mandatory instance each item depends on, strictest kept.
    let mut rows = Vec::new();
    for item in &sub.items {
        let deps = state
            .checklist
            .iter()
            .find(|c| c.id == item.id)
            .map(|c| c.instance_ids.clone())
            .unwrap_or_default();
        let row = aggregate::strictest(
            state
                .instances
                .iter()
                .filter(|i| deps.contains(&i.id()))
                .map(|i| aggregate::row(i, item.disposition, item.confidence)),
        );
        rows.push(row);
        state.items.push(ItemRow {
            id: item.id.clone(),
            disposition: item.disposition,
            confidence: item.confidence,
            reason: item.reason.clone(),
            location: item.location.clone(),
            row,
        });
    }
    let rows: Vec<Row> = rows;
    Ok(aggregate::verdict(&rows))
}

/// Recovers the image id from the label block built above. Kept local to the walking slice.
fn image_id_of(label: &ContentBlock) -> String {
    match label {
        ContentBlock::Text { text } => text.split('`').nth(1).unwrap_or_default().to_string(),
        _ => String::new(),
    }
}

fn new_run_dir(out_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dir = out_dir.join(format!("run-{unix_ms}-{}", std::process::id()));
    std::fs::create_dir(&dir)?;
    Ok(dir)
}

/// What the rules resolved to, for the report and the cache key.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RulesSummary {
    pub base: String,
    pub profile: Option<String>,
    pub versions: std::collections::BTreeMap<String, u32>,
    pub obligations: Vec<obligations::MergedObligation>,
    pub warnings: Vec<String>,
}

/// Rules and obligations resolved for a job, before any model call.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub summary: RulesSummary,
    pub resolved: Resolved,
    pub system_prompt: String,
    pub checklist: Vec<ChecklistItem>,
}

/// Merges and resolves the rules for a job. `[rules]` in the config names the base; without it the
/// base is the job's use case.
pub fn resolve_rules(config: &Config, job: &Job, job_base: &Path) -> Result<Prepared> {
    let (base, profile, extra_paths): (String, Option<String>, Vec<PathBuf>) = match &config.rules {
        Some(r) => (r.base.clone(), r.profile.clone(), r.extra.clone()),
        None => (job.use_case.clone(), None, Vec::new()),
    };
    let mut extra = Vec::new();
    let mut names = Vec::new();
    for path in &extra_paths {
        let full = if path.is_absolute() {
            path.clone()
        } else {
            job_base.join(path)
        };
        extra.push(RulesFile::load(&full)?);
        names.push(path.display().to_string());
    }
    let merged = obligations::merge(&MergeInput {
        base: &base,
        profile: profile.as_deref(),
        extra: &extra,
        extra_names: &names,
        job: Some(job),
        enabled_tools: &config.tools.enabled,
    })?;
    let resolved = obligations::resolve(&merged, job)?;
    let checklist = checklist::build(&resolved.instances);
    let system_prompt = render::system_prompt(&resolved, job);
    Ok(Prepared {
        summary: RulesSummary {
            base: merged.base.clone(),
            profile: merged.profile.clone(),
            versions: merged.versions.clone(),
            obligations: merged.obligations.clone(),
            warnings: merged.warnings.clone(),
        },
        resolved,
        system_prompt,
        checklist,
    })
}
