//! The run: resolve the job, show it to the model, take the submission, aggregate, record. A
//! decision is one or two independent runs combined (`decision`). Tool turns, the final turn,
//! repairs, the ledger and the cache arrive in tasks 1.7a to 1.11.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::aggregate::{self, Verdict};
use crate::checklist::Checklist;
use crate::config::Config;
use crate::decision::{self, Decision, DecisionItem, RunSummary};
use crate::frames::Registry;
use crate::image::{self, CanonicalImage};
use crate::job::Job;
use crate::journal::Journal;
use crate::obligations::{self, Instance, MergeInput, Resolved, RulesFile};
use crate::provider::{ContentBlock, Message, MessageRole, Provider, StopReason, TurnRequest};
use crate::render;
use crate::report::{ErrorEntry, InstanceEntry, ItemRow, REPORT_VERSION, Report, RunReport};
use crate::submission::{self, ValidationContext};
use crate::{Error, Result};

/// What `check` produced: a decision and where its directory is.
#[derive(Debug)]
pub struct Outcome {
    pub dir: PathBuf,
    pub decision: Decision,
    /// The first run's error, when the decision is an error.
    pub error: Option<String>,
    pub exit_code: i32,
}

/// Exit code for a decision that ended in an error under the default policy.
pub const EXIT_ERROR: i32 = 3;

/// Runs one decision (`budget.decision_runs` independent runs) and writes its report under
/// `out_dir/<decision dir>/`, with each run's journal under `run-<n>/`.
pub async fn check(
    config: &Config,
    job: &Job,
    job_base: &Path,
    provider: &dyn Provider,
    out_dir: &Path,
    harness_version: &str,
) -> Result<Outcome> {
    let dir = new_decision_dir(out_dir)?;
    let prepared = resolve_rules(config, job, job_base);

    let mut runs: Vec<RunReport> = Vec::new();
    let mut summaries: Vec<RunSummary> = Vec::new();
    let mut first_error: Option<String> = None;
    for n in 1..=config.budget.decision_runs {
        let run_dir = dir.join(format!("run-{n}"));
        std::fs::create_dir(&run_dir)?;
        let (report, summary, error) =
            run_once(config, job, job_base, provider, &prepared, &run_dir, n).await?;
        if first_error.is_none() {
            first_error = error;
        }
        runs.push(report);
        summaries.push(summary);
    }

    let decision = match summaries.as_slice() {
        [one] => decision::single(one),
        [a, b] => decision::combine(a, b),
        _ => return Err(Error::Config("decision_runs must be 1 or 2".into())),
    };
    let exit_code = decision::exit_code(decision.verdict, &config.policy);
    let rules_summary = prepared.as_ref().ok().map(|p| p.summary.clone());
    let report = Report {
        report_version: REPORT_VERSION,
        harness_version: harness_version.to_string(),
        use_case: job.use_case.clone(),
        job: job.summary(),
        config: config.redacted_json(),
        rules: rules_summary,
        provider: provider.id().to_string(),
        decision_runs: config.budget.decision_runs,
        runs,
        decision: decision.clone(),
        verdict: decision.verdict,
        policy: config.policy.clone(),
        exit_code,
    };
    report.write(&dir)?;
    Ok(Outcome {
        dir,
        decision,
        error: first_error,
        exit_code,
    })
}

/// One run of the state machine. Never fails for run-level reasons: those become the run's error
/// entry; only I/O on the journal propagates.
async fn run_once(
    config: &Config,
    job: &Job,
    job_base: &Path,
    provider: &dyn Provider,
    prepared: &Result<Prepared>,
    run_dir: &Path,
    n: u32,
) -> Result<(RunReport, RunSummary, Option<String>)> {
    let mut journal = Journal::create(run_dir)?;
    let mut state = RunState::default();
    let result = match prepared {
        Ok(p) => drive(config, job, job_base, provider, p, &mut journal, &mut state).await,
        Err(e) => Err(Error::Config(e.to_string())),
    };
    let (verdict, error) = match result {
        Ok(v) => {
            journal.append("verdict", serde_json::json!({ "verdict": v }))?;
            (Some(v), None)
        }
        Err(e) => {
            journal.append(
                "error",
                serde_json::json!({ "class": e.class(), "message": e.to_string() }),
            )?;
            (None, Some(e))
        }
    };
    let error_text = error.as_ref().map(ToString::to_string);
    let report = RunReport {
        run: n,
        images: state.images.clone(),
        instances: state
            .instances
            .iter()
            .map(|i| InstanceEntry {
                id: i.id(),
                instance: i.clone(),
            })
            .collect(),
        checklist: state.checklist.clone(),
        items: state.items.clone(),
        verdict,
        error: error.map(|e| ErrorEntry {
            class: e.class().into(),
            message: e.to_string(),
        }),
    };
    let summary = RunSummary {
        verdict,
        items: state
            .items
            .iter()
            .map(|row| {
                let entry = state.checklist.get(&row.id);
                DecisionItem {
                    id: row.id.clone(),
                    disposition: row.disposition,
                    confidence: row.confidence,
                    allowed_by: row.allowed_by.clone(),
                    root_image: entry.and_then(|c| c.root_image.clone()),
                    instance_ids: entry.map(|c| c.instance_ids.clone()).unwrap_or_default(),
                    root_location: entry.and_then(|c| c.root_location.clone()),
                }
            })
            .collect(),
    };
    Ok((report, summary, error_text))
}

#[derive(Default)]
struct RunState {
    images: Vec<CanonicalImage>,
    instances: Vec<Instance>,
    checklist: Checklist,
    registry: Registry,
    items: Vec<ItemRow>,
}

async fn drive(
    config: &Config,
    job: &Job,
    job_base: &Path,
    provider: &dyn Provider,
    prepared: &Prepared,
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
    state.instances = prepared.resolved.instances.clone();
    state.checklist = prepared.checklist.clone();
    for img in &state.images {
        state
            .registry
            .add_raw(&img.id, img.width, img.height, img.projection);
    }
    journal.append(
        "resolved",
        serde_json::json!({
            "images": state.images.iter().map(|i| serde_json::json!({"id": i.id, "role": i.role, "sha256": i.sha256, "canonical_sha256": i.canonical_sha256, "width": i.width, "height": i.height, "projection": i.projection, "warnings": i.warnings})).collect::<Vec<_>>(),
            "instances": state.instances.iter().map(Instance::id).collect::<Vec<_>>(),
            "checklist": state.checklist.ids().collect::<Vec<_>>(),
        }),
    )?;

    // First look: every image, the checklist, the instruction to submit.
    let mut content: Vec<ContentBlock> = Vec::new();
    for img in &state.images {
        content.push(ContentBlock::Text {
            text: format!(
                "Image `{}` (role {:?}{}, frame raw:{}, {}x{}, {:?}).",
                img.id,
                img.role,
                img.label
                    .as_deref()
                    .map(|l| format!(", {l}"))
                    .unwrap_or_default(),
                img.id,
                img.width,
                img.height,
                img.projection
            ),
        });
        content.push(ContentBlock::Image {
            image_id: img.id.clone(),
        });
    }
    content.push(ContentBlock::Text {
        text: state.checklist.render(&state.instances),
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
        system: &prepared.system_prompt,
        history: &history,
        tools: &tools,
        max_output_tokens: config.provider.max_output_tokens,
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

    // Until task 1.7a, exactly one submit_verdict call is accepted and nothing else.
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
    let completed: Vec<String> = Vec::new();
    let sub = submission::parse_and_validate(
        &call.arguments,
        &ValidationContext {
            checklist: &state.checklist,
            resolved: &prepared.resolved,
            registry: &state.registry,
            completed_calls: &completed,
        },
    )?;
    journal.append("submission", serde_json::json!({ "call_id": call.id, "items": sub.items.len(), "findings": sub.findings.len() }))?;

    // Aggregate: one row per mandatory instance each item depends on, strictest kept.
    let mut rows = Vec::new();
    for item in &sub.items {
        let deps = state
            .checklist
            .get(&item.id)
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
            allowed_by: item.allowed_by.clone(),
            location: item.location.clone(),
            row,
        });
    }
    for f in &sub.findings {
        // Findings contribute rows through the mandatory instances they cite on their image.
        let d: aggregate::Disposition = f.disposition.into();
        if d == aggregate::Disposition::Observed {
            continue;
        }
        let root = state.registry.to_root(&f.location).ok().and_then(|m| {
            state
                .registry
                .get(&m.location.frame)
                .map(|fr| fr.root().to_string())
        });
        let row = aggregate::strictest(
            state
                .instances
                .iter()
                .filter(|i| {
                    f.obligation_ids.contains(&i.obligation_id) && Some(&i.target) == root.as_ref()
                })
                .map(|i| aggregate::row(i, d, f.confidence)),
        );
        rows.push(row);
    }
    Ok(aggregate::verdict(&rows))
}

fn new_decision_dir(out_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dir = out_dir.join(format!("decision-{unix_ms}-{}", std::process::id()));
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
    pub checklist: Checklist,
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
    let images: Vec<(String, crate::job::Role)> =
        job.images.iter().map(|i| (i.id.clone(), i.role)).collect();
    let checklist = Checklist::build(&resolved, &images)?;
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
