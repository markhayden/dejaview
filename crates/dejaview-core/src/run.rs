//! The run state machine (SPEC "Run state machine"): resolve, prepare, reserve, tool turns, the
//! final turn, aggregate, record. A decision is one or two independent runs combined (`decision`).
//! Task 1.7a: tool turns with batch validation, the call budget, result ingestion. Task 1.7b: the
//! final turn's refuse mechanism and every terminal error class. Task 1.8: both repair forms and
//! transport retries. Task 1.9: the cost ledger. Tasks 1.10 and 1.11: journal artifacts, cache.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::aggregate::{self, Verdict};
use crate::checklist::Checklist;
use crate::config::Config;
use crate::decision::{self, Decision, DecisionItem, RunSummary};
use crate::frames::Registry;
use crate::image::{self, CanonicalImage};
use crate::job::Job;
use crate::journal::Journal;
use crate::ledger::{self, Account, CapLedger, Ledger, Pricing, RequestShape};
use crate::obligations::{self, Instance, MergeInput, Resolved, RulesFile};
use crate::provider::{
    ContentBlock, Message, MessageRole, Provider, StopReason, ToolCall, ToolDefinition,
    TurnRequest, TurnResponse,
};
use crate::render;
use crate::report::{
    CostSummary, ErrorEntry, InstanceEntry, ItemRow, REPORT_VERSION, RepairEntry, Report,
    RunReport, ToolCallEntry, TurnEntry,
};
use crate::retry;
use crate::submission::{self, ValidationContext};
use crate::tool::{
    self, ArgCheck, DejaviewContext, FrameHandle, ImageHandle, Status, ToolRegistry,
};
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

/// The reserved tool the model submits with.
pub const SUBMIT: &str = "submit_verdict";

/// Runs one decision (`budget.decision_runs` independent runs) and writes its report under
/// `out_dir/<decision dir>/`, with each run's journal under `run-<n>/`. The decision envelope is
/// reserved against a per-job ledger capped at `budget.spend_cap_usd` and, when given, a `shared`
/// ledger (the evaluation's).
#[allow(clippy::too_many_arguments)]
pub async fn check(
    config: &Config,
    job: &Job,
    job_base: &Path,
    provider: &dyn Provider,
    tools: &ToolRegistry,
    out_dir: &Path,
    harness_version: &str,
    shared: Option<&dyn Ledger>,
) -> Result<Outcome> {
    let dir = new_decision_dir(out_dir)?;
    let prepared = resolve_rules(config, job, job_base, tools);
    let job_ledger = CapLedger::new(config.budget.spend_cap_usd);
    let envelope = match (&prepared, provider.pricing()) {
        (Ok(p), Some(pricing)) => Some(ledger::plan(config, &plan_input(p, job, tools), pricing)),
        (Ok(_), None) => Some(Err(Error::Budget(
            "the provider cannot bound request costs; refusing to run".into(),
        ))),
        (Err(_), _) => None,
    };
    let mut money = Money {
        envelope: &envelope,
        pricing: provider.pricing(),
        job_ledger: &job_ledger,
        shared,
        account: None,
    };

    let mut runs: Vec<RunReport> = Vec::new();
    let mut summaries: Vec<RunSummary> = Vec::new();
    let mut first_error: Option<String> = None;
    for n in 1..=config.budget.decision_runs {
        let run_dir = dir.join(format!("run-{n}"));
        std::fs::create_dir(&run_dir)?;
        let (report, summary, error) = run_once(
            config, job, job_base, provider, tools, &prepared, &run_dir, n, &mut money,
        )
        .await?;
        if first_error.is_none() {
            first_error = error;
        }
        runs.push(report);
        summaries.push(summary);
    }
    let attempts = runs
        .iter()
        .flat_map(|r| r.turns.iter())
        .map(|t| t.attempts)
        .sum();
    let cost = match money.account.take() {
        Some(account) => {
            let closing = account.close();
            job_ledger.settle(closing.envelope_usd, closing.spent_usd);
            if let Some(s) = shared {
                s.settle(closing.envelope_usd, closing.spent_usd);
            }
            CostSummary {
                envelope: envelope.as_ref().and_then(|e| e.as_ref().ok().cloned()),
                cap_usd: config.budget.spend_cap_usd,
                spent_usd: closing.spent_usd,
                released_usd: closing.released_usd,
                overrun_usd: closing.overrun_usd,
                attempts,
            }
        }
        None => CostSummary {
            envelope: envelope.as_ref().and_then(|e| e.as_ref().ok().cloned()),
            cap_usd: config.budget.spend_cap_usd,
            attempts,
            ..CostSummary::default()
        },
    };

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
        prompt_version: render::PROMPT_VERSION.to_string(),
        provider: provider.id().to_string(),
        tools: tools
            .manifests()
            .map(|m| format!("{}@{}", m.name, m.version))
            .collect(),
        decision_runs: config.budget.decision_runs,
        runs,
        decision: decision.clone(),
        verdict: decision.verdict,
        policy: config.policy.clone(),
        exit_code,
        cost,
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
#[allow(clippy::too_many_arguments)]
async fn run_once(
    config: &Config,
    job: &Job,
    job_base: &Path,
    provider: &dyn Provider,
    tools: &ToolRegistry,
    prepared: &Result<Prepared>,
    run_dir: &Path,
    n: u32,
    money: &mut Money<'_>,
) -> Result<(RunReport, RunSummary, Option<String>)> {
    let mut journal = Journal::create(run_dir)?;
    let mut state = RunState::new(run_dir, n);
    let result = match prepared {
        Ok(p) => {
            drive(
                config,
                job,
                job_base,
                provider,
                tools,
                p,
                &mut journal,
                &mut state,
                money,
            )
            .await
        }
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
        turns: state.turns.clone(),
        tool_calls: state.tool_calls.clone(),
        repairs: state.repairs.clone(),
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

/// The decision's money: the planned envelope, the ledgers it is reserved against, and the
/// account once reserved (by the first run).
struct Money<'a> {
    envelope: &'a Option<Result<ledger::Envelope>>,
    pricing: Option<&'a dyn Pricing>,
    job_ledger: &'a CapLedger,
    shared: Option<&'a dyn Ledger>,
    account: Option<Account>,
}

/// What the envelope plan needs from the resolved rules, the job and the tools.
fn plan_input(prepared: &Prepared, job: &Job, tools: &ToolRegistry) -> ledger::PlanInput {
    let checklist_bytes = prepared
        .checklist
        .render(&prepared.resolved.instances)
        .len() as u64;
    let tools_bytes: u64 = tools
        .manifests()
        .map(|m| {
            serde_json::to_string(&m.tool_definition())
                .map(|s| s.len() as u64)
                .unwrap_or(0)
        })
        .sum::<u64>()
        + serde_json::to_string(&submission::submit_verdict_tool())
            .map(|s| s.len() as u64)
            .unwrap_or(0);
    ledger::PlanInput {
        system_bytes: prepared.system_prompt.len() as u64,
        tools_bytes,
        // Per image: its label line; then the checklist and the instruction.
        first_look_bytes: job.images.len() as u64 * 160 + checklist_bytes + 400,
        checklist_bytes,
        job_images: job.images.len() as u64,
        tool_envelope_bytes: tools
            .manifests()
            .map(|m| m.returns.max_envelope_bytes)
            .max()
            .unwrap_or(0),
        tool_images: tools
            .manifests()
            .map(|m| u64::from(m.returns.max_images))
            .max()
            .unwrap_or(0),
    }
}

/// The shape of a request as sent, for its attempt's bound.
fn shape_of(request: &TurnRequest<'_>) -> RequestShape {
    let mut bytes = request.system.len() as u64
        + serde_json::to_string(request.tools)
            .map(|s| s.len() as u64)
            .unwrap_or(0);
    let mut images: u64 = 0;
    for m in request.history {
        for b in &m.content {
            match b {
                ContentBlock::Text { text } => bytes += text.len() as u64,
                ContentBlock::Image { .. } => images += 1,
                ContentBlock::ToolUse {
                    name, arguments, ..
                } => bytes += name.len() as u64 + arguments.to_string().len() as u64,
                ContentBlock::ToolResult {
                    content,
                    images: imgs,
                    ..
                } => {
                    bytes += content.len() as u64;
                    images += imgs.len() as u64;
                }
            }
        }
    }
    RequestShape {
        input_tokens: ledger::estimated_tokens(bytes),
        output_tokens: u64::from(request.max_output_tokens),
        images,
    }
}

struct RunState {
    run_dir: PathBuf,
    run_id: String,
    images: Vec<CanonicalImage>,
    instances: Vec<Instance>,
    checklist: Checklist,
    registry: Registry,
    /// Lossless pixels per frame id, written under the run directory for tools.
    frame_files: BTreeMap<String, PathBuf>,
    turns: Vec<TurnEntry>,
    tool_calls: Vec<ToolCallEntry>,
    completed_calls: Vec<String>,
    repairs: Vec<RepairEntry>,
    items: Vec<ItemRow>,
    next_call: u32,
}

impl RunState {
    fn new(run_dir: &Path, n: u32) -> RunState {
        RunState {
            run_dir: run_dir.to_path_buf(),
            run_id: format!("run-{n}"),
            images: Vec::new(),
            instances: Vec::new(),
            checklist: Checklist::default(),
            registry: Registry::new(),
            frame_files: BTreeMap::new(),
            turns: Vec::new(),
            tool_calls: Vec::new(),
            completed_calls: Vec::new(),
            repairs: Vec::new(),
            items: Vec::new(),
            next_call: 1,
        }
    }

    fn next_call_id(&mut self) -> String {
        let id = format!("call:{}", self.next_call);
        self.next_call += 1;
        id
    }

    /// The pixels of a frame as a file under `frames/`: raw frames are written from the canonical
    /// pixels on first use; derived frames were written when the tool returned them.
    fn frame_file(&mut self, frame_id: &str) -> Result<PathBuf> {
        if let Some(p) = self.frame_files.get(frame_id) {
            return Ok(p.clone());
        }
        let image_id = frame_id
            .strip_prefix("raw:")
            .ok_or_else(|| Error::Tool(format!("no pixels stored for frame {frame_id:?}")))?;
        let img = self
            .images
            .iter()
            .find(|i| i.id == image_id)
            .ok_or_else(|| Error::Tool(format!("unknown image {image_id:?}")))?;
        let pixels = img
            .pixels
            .clone()
            .ok_or_else(|| Error::Tool(format!("image {image_id:?} has no pixels")))?;
        let dir = self.run_dir.join("frames");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.png", sanitise(frame_id)));
        pixels
            .save(&path)
            .map_err(|e| Error::Tool(format!("cannot write {}: {e}", path.display())))?;
        self.frame_files.insert(frame_id.to_string(), path.clone());
        Ok(path)
    }
}

fn sanitise(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
async fn drive(
    config: &Config,
    job: &Job,
    job_base: &Path,
    provider: &dyn Provider,
    tools: &ToolRegistry,
    prepared: &Prepared,
    journal: &mut Journal,
    state: &mut RunState,
    money: &mut Money<'_>,
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
            "tools": tools.manifests().map(|m| m.name.clone()).collect::<Vec<_>>(),
        }),
    )?;
    // Prepare is a stub until task 2.3; the event exists so the journal's shape is final.
    journal.append(
        "prepared",
        serde_json::json!({ "prep_version": "canonical", "images": state.images.len() }),
    )?;

    // Reserve: the decision envelope, once per decision, before any request.
    let envelope = match money.envelope {
        Some(Ok(e)) => e,
        Some(Err(e)) => return Err(Error::Budget(e.to_string())),
        None => return Err(Error::Budget("no envelope was planned".into())),
    };
    let Some(pricing) = money.pricing else {
        return Err(Error::Budget(
            "the provider cannot bound request costs".into(),
        ));
    };
    if money.account.is_none() {
        money.job_ledger.reserve(envelope.usd)?;
        if let Some(shared) = money.shared
            && let Err(e) = shared.reserve(envelope.usd)
        {
            money.job_ledger.settle(envelope.usd, 0.0);
            return Err(e);
        }
        money.account = Some(Account::new(envelope.usd));
    }
    journal.append(
        "reserved",
        serde_json::json!({
            "envelope_usd": envelope.usd,
            "cap_usd": config.budget.spend_cap_usd,
            "shared_cap_usd": money.shared.map(Ledger::cap),
            "requests_per_run": envelope.requests.len(),
            "attempts": envelope.attempts,
        }),
    )?;
    let Some(account) = money.account.as_mut() else {
        return Err(Error::Budget("the envelope was not reserved".into()));
    };

    // Tool definitions are declared once and never change for the conversation.
    let mut definitions: Vec<ToolDefinition> =
        tools.manifests().map(|m| m.tool_definition()).collect();
    definitions.push(submission::submit_verdict_tool());

    // First look.
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
        text: if tools.is_empty() {
            "Inspect the images, then call submit_verdict with one disposition per checklist item.".into()
        } else {
            format!(
                "Inspect the images, use tools where a closer look would change your answer (at most {} calls over {} turns), then call submit_verdict with one disposition per checklist item.",
                config.budget.tool_calls, config.budget.tool_turns
            )
        },
    });
    let mut history = vec![Message {
        role: MessageRole::User,
        content,
    }];

    let mut calls_used: u32 = 0;
    let mut turns_used: u32 = 0;
    let mut allowance = config.provider.max_output_tokens;
    while turns_used < config.budget.tool_turns {
        let response = send_turn(
            config,
            provider,
            prepared,
            &history,
            &definitions,
            allowance,
            journal,
            state,
            pricing,
            account,
        )
        .await?;
        allowance = config.provider.max_output_tokens;

        match classify(&response.stop_reason, &response.tool_calls) {
            Class::Submission(call) => {
                history.push(assistant_message(&response.text, &response.tool_calls));
                match finish(prepared, state, journal, call) {
                    Ok(verdict) => return Ok(verdict),
                    Err(Error::Submission(message)) => {
                        if !take_repair(config, state, journal, "rejected", "invalid_submission")? {
                            return Err(Error::Submission(message));
                        }
                        history.push(rejection(
                            std::slice::from_ref(call),
                            &format!("Submission rejected: {message}"),
                            None,
                        ));
                    }
                    Err(e) => return Err(e),
                }
            }
            Class::MixedCalls(calls) => {
                history.push(assistant_message(&response.text, &response.tool_calls));
                if !take_repair(config, state, journal, "rejected", "mixed_batch")? {
                    return Err(Error::ModelOutput(
                        "submit_verdict must be the only call in its batch (mixed batch), again after the repair".into(),
                    ));
                }
                history.push(rejection(
                    calls,
                    &format!("Not executed: {SUBMIT} must be the only call in its batch."),
                    None,
                ));
            }
            Class::Excluded(reason) => {
                exclude(journal, state, &response, reason)?;
                if !take_repair(config, state, journal, "reissue", reason.name())? {
                    return Err(Error::ModelOutput(format!(
                        "{} (stop reason {:?}), again after the repair",
                        reason.describe(),
                        response.stop_reason
                    )));
                }
                match reason {
                    Excluded::Truncated => allowance = config.budget.repair_output_tokens,
                    Excluded::Refusal | Excluded::NoCall => history.push(discarded_notice(None)),
                }
            }
            Class::Batch(calls) => {
                history.push(assistant_message(&response.text, &response.tool_calls));
                turns_used += 1;
                let remaining = config.budget.tool_calls.saturating_sub(calls_used);
                if calls.len() as u32 > remaining {
                    // Execute none; answer every call; go to the final turn.
                    let results: Vec<ContentBlock> = calls
                        .iter()
                        .map(|c| ContentBlock::ToolResult {
                            tool_use_id: c.id.clone(),
                            content: format!(
                                "Not executed: this batch of {} calls exceeds the remaining call budget of {remaining}. Submit your verdict now.",
                                calls.len()
                            ),
                            is_error: true,
                            images: vec![],
                        })
                        .collect();
                    for c in calls {
                        journal.append(
                            "tool_called",
                            serde_json::json!({ "tool": c.name, "provider_call_id": c.id, "executed": false, "reason": "call budget" }),
                        )?;
                    }
                    history.push(Message {
                        role: MessageRole::User,
                        content: results,
                    });
                    break;
                }
                let mut results: Vec<ContentBlock> = Vec::new();
                for c in calls {
                    results.push(execute_call(config, tools, prepared, journal, state, c)?);
                }
                calls_used += calls.len() as u32;
                results.push(ContentBlock::Text {
                    text: state.checklist.render(&state.instances),
                });
                history.push(Message {
                    role: MessageRole::User,
                    content: results,
                });
                if calls_used >= config.budget.tool_calls {
                    break;
                }
            }
        }
    }

    // Final turn, `refuse` mechanism: the same frozen tool definitions, an appended instruction,
    // and only submit_verdict is executed. A stray call is answered with a provider-native error
    // result and consumes the repair; a repair after the final turn carries the same instruction.
    // Then `error: no_submission`.
    let instruction = final_turn_instruction(state);
    history.push(Message {
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: instruction.clone(),
        }],
    });
    loop {
        let response = send_turn(
            config,
            provider,
            prepared,
            &history,
            &definitions,
            allowance,
            journal,
            state,
            pricing,
            account,
        )
        .await?;
        allowance = config.provider.max_output_tokens;
        match classify(&response.stop_reason, &response.tool_calls) {
            Class::Submission(call) => {
                history.push(assistant_message(&response.text, &response.tool_calls));
                match finish(prepared, state, journal, call) {
                    Ok(verdict) => return Ok(verdict),
                    Err(Error::Submission(message)) => {
                        if !take_repair(config, state, journal, "rejected", "invalid_submission")? {
                            return Err(Error::Submission(message));
                        }
                        history.push(rejection(
                            std::slice::from_ref(call),
                            &format!("Submission rejected: {message}"),
                            Some(&instruction),
                        ));
                    }
                    Err(e) => return Err(e),
                }
            }
            Class::Batch(calls) | Class::MixedCalls(calls) => {
                history.push(assistant_message(&response.text, &response.tool_calls));
                for c in calls {
                    journal.append(
                        "tool_called",
                        serde_json::json!({ "tool": c.name, "provider_call_id": c.id, "executed": false, "reason": "final_turn" }),
                    )?;
                }
                if !take_repair(config, state, journal, "rejected", "stray_call")? {
                    return Err(Error::NoSubmission(format!(
                        "the final turn called {:?} instead of submitting, again after the repair",
                        calls.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()
                    )));
                }
                history.push(rejection(
                    calls,
                    &format!("Not available: only {SUBMIT} is executed on the final turn."),
                    Some(&instruction),
                ));
            }
            Class::Excluded(reason) => {
                exclude(journal, state, &response, reason)?;
                if !take_repair(config, state, journal, "reissue", reason.name())? {
                    return Err(Error::NoSubmission(format!(
                        "the final turn ended with {} (stop reason {:?}), again after the repair",
                        reason.describe(),
                        response.stop_reason
                    )));
                }
                match reason {
                    Excluded::Truncated => allowance = config.budget.repair_output_tokens,
                    Excluded::Refusal | Excluded::NoCall => {
                        history.push(discarded_notice(Some(&instruction)));
                    }
                }
            }
        }
    }
}

/// The appended final-turn instruction, with the full checklist.
fn final_turn_instruction(state: &RunState) -> String {
    format!(
        "Tool budget exhausted. Only {SUBMIT} will be executed now; any other call is answered with an error. \
         Submit your verdict with exactly one disposition per checklist item.\n\n{}",
        state.checklist.render(&state.instances)
    )
}

/// Spends the run's repair if one is left, journaling it.
fn take_repair(
    config: &Config,
    state: &mut RunState,
    journal: &mut Journal,
    form: &str,
    reason: &str,
) -> Result<bool> {
    if state.repairs.len() as u32 >= config.budget.repairs {
        return Ok(false);
    }
    let entry = RepairEntry {
        form: form.into(),
        reason: reason.into(),
        after_turn: state.turns.len() as u32,
    };
    journal.append(
        "repair",
        serde_json::json!({ "form": entry.form, "reason": entry.reason, "after_turn": entry.after_turn, "repairs_used": state.repairs.len() + 1 }),
    )?;
    state.repairs.push(entry);
    Ok(true)
}

/// Journals a non-replayable response and marks its turn as never having entered provider history.
fn exclude(
    journal: &mut Journal,
    state: &mut RunState,
    response: &TurnResponse,
    reason: Excluded,
) -> Result<()> {
    let turn = state.turns.len() as u32;
    journal.append(
        "turn_excluded",
        serde_json::json!({ "turn": turn, "stop_reason": response.stop_reason, "reason": reason.name(), "text_blocks": response.text.len(), "tool_calls": response.tool_calls.len() }),
    )?;
    if let Some(entry) = state.turns.last_mut() {
        entry.entered_history = false;
    }
    Ok(())
}

/// The repair message for a replayable response that was rejected: one error result per call, in
/// order, then the repair instruction (and the final-turn instruction again when inside it).
fn rejection(calls: &[ToolCall], reason: &str, final_instruction: Option<&str>) -> Message {
    let mut content: Vec<ContentBlock> = calls
        .iter()
        .map(|c| ContentBlock::ToolResult {
            tool_use_id: c.id.clone(),
            content: reason.to_string(),
            is_error: true,
            images: vec![],
        })
        .collect();
    content.push(ContentBlock::Text {
        text: format!(
            "Repair: call {SUBMIT} once, alone, with exactly one disposition per checklist item. This is the only repair.{}",
            final_instruction.map(|i| format!("\n\n{i}")).unwrap_or_default()
        ),
    });
    Message {
        role: MessageRole::User,
        content,
    }
}

/// The reissue message after a refusal or a call-less text: the previous output was discarded.
fn discarded_notice(final_instruction: Option<&str>) -> Message {
    Message {
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: format!(
                "Your previous output was discarded: it contained no tool call. Inspect the images and call a tool, or call {SUBMIT} with exactly one disposition per checklist item.{}",
                final_instruction
                    .map(|i| format!("\n\n{i}"))
                    .unwrap_or_default()
            ),
        }],
    }
}

#[derive(Debug)]
enum Class<'a> {
    Submission(&'a ToolCall),
    Batch(&'a [ToolCall]),
    /// A batch that mixes submit_verdict with other calls.
    MixedCalls(&'a [ToolCall]),
    /// A non-replayable response: never enters provider history.
    Excluded(Excluded),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Excluded {
    Truncated,
    Refusal,
    NoCall,
}

impl Excluded {
    fn name(self) -> &'static str {
        match self {
            Excluded::Truncated => "truncated",
            Excluded::Refusal => "refusal",
            Excluded::NoCall => "no_call",
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Excluded::Truncated => "the response was truncated",
            Excluded::Refusal => "the model refused",
            Excluded::NoCall => "the model answered without calling a tool",
        }
    }
}

fn classify<'a>(stop: &StopReason, calls: &'a [ToolCall]) -> Class<'a> {
    match stop {
        // Never dispatched, even if a call inside parses.
        StopReason::MaxTokens => return Class::Excluded(Excluded::Truncated),
        StopReason::Refusal => return Class::Excluded(Excluded::Refusal),
        _ => {}
    }
    let submits = calls.iter().filter(|c| c.name == SUBMIT).count();
    match (calls.len(), submits) {
        (0, _) => Class::Excluded(Excluded::NoCall),
        (1, 1) => Class::Submission(&calls[0]),
        (_, 0) => Class::Batch(calls),
        _ => Class::MixedCalls(calls),
    }
}

fn assistant_message(text: &[String], calls: &[ToolCall]) -> Message {
    let mut content: Vec<ContentBlock> = text
        .iter()
        .map(|t| ContentBlock::Text { text: t.clone() })
        .collect();
    for c in calls {
        content.push(ContentBlock::ToolUse {
            id: c.id.clone(),
            name: c.name.clone(),
            arguments: c.arguments.clone(),
        });
    }
    Message {
        role: MessageRole::Assistant,
        content,
    }
}

/// Sends one request with transport retries, journaling the attempt and the response.
#[allow(clippy::too_many_arguments)]
async fn send_turn(
    config: &Config,
    provider: &dyn Provider,
    prepared: &Prepared,
    history: &[Message],
    definitions: &[ToolDefinition],
    max_output_tokens: u32,
    journal: &mut Journal,
    state: &mut RunState,
    pricing: &dyn Pricing,
    account: &mut Account,
) -> Result<TurnResponse> {
    let turn = state.turns.len() as u32 + 1;
    let request = TurnRequest {
        system: &prepared.system_prompt,
        history,
        tools: definitions,
        max_output_tokens,
    };
    let bound_usd = pricing.bound(&shape_of(&request))?;
    journal.append(
        "turn_sent",
        serde_json::json!({ "turn": turn, "tools": definitions.iter().map(|t| t.name.clone()).collect::<Vec<_>>(), "history_messages": history.len(), "max_output_tokens": max_output_tokens, "bound_usd": bound_usd }),
    )?;
    // A failed attempt's usage is unknown: its hold is charged at the bound.
    let mut charged_usd = 0.0;
    let mut on_retry = |a: &retry::Attempt| -> Result<()> {
        let hold = account.hold(bound_usd);
        account.charge(hold);
        charged_usd += bound_usd;
        journal.append(
            "retry",
            serde_json::json!({ "turn": turn, "attempt": a.attempt, "status": a.status, "delay_ms": a.delay.as_millis() as u64, "error": a.message, "charged_usd": bound_usd }),
        )
    };
    let sent = retry::send_with_retries(
        provider,
        request,
        config.budget.retries,
        &retry::Backoff::default(),
        &retry::TokioSleeper,
        &mut rand::random::<f64>,
        &mut on_retry,
    )
    .await?;
    let response = sent.response;
    let hold = account.hold(bound_usd);
    let settled_usd = pricing.cost(&response.usage);
    account.settle(hold, settled_usd);
    let cost_usd = settled_usd + charged_usd;
    journal.append(
        "turn_received",
        serde_json::json!({
            "turn": turn,
            "stop_reason": response.stop_reason,
            "tool_calls": response.tool_calls.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
            "usage": response.usage,
            "attempts": sent.attempts,
            "cost_usd": cost_usd,
        }),
    )?;
    state.turns.push(TurnEntry {
        turn,
        stop_reason: response.stop_reason.clone(),
        tool_calls: response.tool_calls.iter().map(|c| c.name.clone()).collect(),
        usage: response.usage,
        provider_request_id: response.provider_request_id.clone(),
        max_output_tokens,
        attempts: sent.attempts,
        bound_usd,
        cost_usd,
        entered_history: true,
    });
    Ok(response)
}

/// Validates and executes one tool call, returning the result block for the model. A model-side
/// mistake is an error result; a tool failure or bound breach is `error: tool`.
fn execute_call(
    config: &Config,
    tools: &ToolRegistry,
    prepared: &Prepared,
    journal: &mut Journal,
    state: &mut RunState,
    call: &ToolCall,
) -> Result<ContentBlock> {
    let call_id = state.next_call_id();
    let started = Instant::now();
    let mut entry = ToolCallEntry {
        id: call_id.clone(),
        provider_call_id: call.id.clone(),
        tool: call.name.clone(),
        arguments: call.arguments.clone(),
        status: None,
        invalid_args: None,
        duration_ms: 0,
        added_items: vec![],
        dropped: vec![],
    };

    // Model-side mistakes become error results, never operational failures.
    macro_rules! reject {
        ($message:expr) => {{
            let message: String = $message;
            entry.invalid_args = Some(message.clone());
            journal.append(
                "tool_called",
                serde_json::json!({ "call_id": entry.id, "tool": entry.tool, "executed": false, "invalid_args": message }),
            )?;
            state.tool_calls.push(entry);
            return Ok(ContentBlock::ToolResult {
                tool_use_id: call.id.clone(),
                content: format!("invalid arguments: {message}"),
                is_error: true,
                images: vec![],
            });
        }};
    }

    let Some(tool) = tools.get(&call.name) else {
        reject!(format!("unknown tool {:?}", call.name));
    };
    let manifest = tool.manifest().clone();
    if let ArgCheck::InvalidArgs(m) = tool::check_model_args(&manifest, &call.arguments) {
        reject!(m);
    }
    // Resolve image and frame arguments through the registry; never a path from the model.
    let mut ctx = DejaviewContext {
        run_id: state.run_id.clone(),
        call_id: call_id.clone(),
        ..Default::default()
    };
    for arg in &manifest.image_args {
        let Some(id) = call.arguments.get(arg).and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(img) = state.images.iter().find(|i| i.id == id).cloned() else {
            reject!(format!("{arg}: unknown image id {id:?}"));
        };
        let path = state.frame_file(&format!("raw:{id}"))?;
        ctx.images.insert(
            id.to_string(),
            ImageHandle {
                media_type: "image/png".into(),
                width: img.width,
                height: img.height,
                projection: img.projection,
                path: Some(path.display().to_string()),
                base64: None,
            },
        );
    }
    for arg in &manifest.frame_args {
        let Some(id) = call.arguments.get(arg).and_then(|v| v.as_str()) else {
            continue;
        };
        let Some((root, projection, frame)) = state
            .registry
            .get(id)
            .map(|f| (f.root().to_string(), f.projection(), f.frame().clone()))
        else {
            reject!(format!("{arg}: unknown frame id {id:?}"));
        };
        let path = match state.frame_file(id) {
            Ok(p) => p,
            Err(_) => reject!(format!("{arg}: frame {id:?} has no pixels available")),
        };
        let (parent, transform) = match &frame {
            crate::frames::Frame::Raw { .. } => (None, None),
            crate::frames::Frame::Aligned {
                parent,
                transform_id,
                ..
            } => (
                Some(parent.clone()),
                Some(serde_json::json!({ "id": transform_id })),
            ),
            crate::frames::Frame::Crop {
                parent, mapping, ..
            } => (Some(parent.clone()), Some(serde_json::to_value(mapping)?)),
        };
        ctx.frames.insert(
            id.to_string(),
            FrameHandle {
                root,
                projection,
                parent,
                transform,
                pixels: ImageHandle {
                    media_type: "image/png".into(),
                    width: 0,
                    height: 0,
                    projection,
                    path: Some(path.display().to_string()),
                    base64: None,
                },
            },
        );
    }
    if let Some(hints) = prepared.hints.get(&manifest.name) {
        ctx.hints = hints.clone();
    }
    journal.append(
        "tool_called",
        serde_json::json!({ "call_id": call_id, "tool": manifest.name, "arguments": call.arguments, "executed": true }),
    )?;

    let mut envelope = tool.call(&call.arguments, &ctx)?;
    entry.duration_ms = started.elapsed().as_millis() as u64;
    entry.status = Some(envelope.status);
    if envelope.status == Status::InvalidArgs {
        let msg = envelope.summary.clone();
        reject!(msg);
    }
    envelope.check_bounds(&manifest.returns)?;

    // Returned images become crop frames with their pixels stored for later calls.
    for img in &envelope.images {
        if state.registry.get(&img.parent).is_none() {
            return Err(Error::Tool(format!(
                "{}: returned image parent {:?} is not a frame",
                manifest.name, img.parent
            )));
        }
        state.registry.add_crop(
            &img.frame,
            &img.parent,
            img.mapping.clone(),
            img.width,
            img.height,
        );
        if let Some(b64) = &img.base64 {
            let bytes = image::base64_decode(b64)
                .map_err(|e| Error::Tool(format!("{}: returned image: {e}", manifest.name)))?;
            let dir = state.run_dir.join("frames");
            std::fs::create_dir_all(&dir)?;
            let ext = if img.media_type == "image/jpeg" {
                "jpg"
            } else {
                "png"
            };
            let path = dir.join(format!("{}.{ext}", sanitise(&img.frame)));
            std::fs::write(&path, bytes)?;
            state.frame_files.insert(img.frame.clone(), path);
        }
    }
    let ingest = state.checklist.ingest(
        &call_id,
        &mut envelope,
        &manifest,
        &prepared.resolved,
        &state.registry,
        config.budget.evidence_items as usize,
    )?;
    for d in &ingest.dropped {
        journal.append(
            "evidence_dropped",
            serde_json::json!({ "call_id": d.call_id, "region_id": d.region_id, "reason": d.reason }),
        )?;
    }
    entry.added_items = ingest.added.clone();
    entry.dropped = ingest.dropped.iter().map(|d| d.region_id.clone()).collect();
    if matches!(envelope.status, Status::Ok | Status::MeasuredNothing) {
        state.completed_calls.push(call_id.clone());
    }
    journal.append(
        "tool_returned",
        serde_json::json!({ "call_id": call_id, "status": envelope.status, "regions": envelope.regions.len(), "images": envelope.images.len(), "added_items": ingest.added, "duration_ms": entry.duration_ms }),
    )?;
    state.tool_calls.push(entry);

    // What the model sees: the envelope minus image bytes, with the checklist ids of its regions.
    let presentation = serde_json::json!({
        "call_id": call_id,
        "status": envelope.status,
        "summary": envelope.summary,
        "measurements": envelope.measurements,
        "regions": envelope.regions.iter().zip(ingest.added.iter()).map(|(r, item)| serde_json::json!({
            "checklist_item": item, "label": r.label, "score": r.score, "location": r.location
        })).collect::<Vec<_>>(),
        "images": envelope.images.iter().map(|i| serde_json::json!({ "frame": i.frame, "parent": i.parent, "mapping": i.mapping })).collect::<Vec<_>>(),
    });
    Ok(ContentBlock::ToolResult {
        tool_use_id: call.id.clone(),
        content: serde_json::to_string(&presentation)?,
        is_error: false,
        images: envelope.images.iter().map(|i| i.frame.clone()).collect(),
    })
}

/// Validates the submission and aggregates the verdict.
fn finish(
    prepared: &Prepared,
    state: &mut RunState,
    journal: &mut Journal,
    call: &ToolCall,
) -> Result<Verdict> {
    let sub = submission::parse_and_validate(
        &call.arguments,
        &ValidationContext {
            checklist: &state.checklist,
            resolved: &prepared.resolved,
            registry: &state.registry,
            completed_calls: &state.completed_calls,
        },
    )?;
    journal.append(
        "submission",
        serde_json::json!({ "provider_call_id": call.id, "items": sub.items.len(), "findings": sub.findings.len() }),
    )?;

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
    pub versions: BTreeMap<String, u32>,
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
    /// Hints per tool, from the merged rules, as JSON for injection.
    pub hints: BTreeMap<String, serde_json::Value>,
}

/// Merges and resolves the rules for a job. `[rules]` in the config names the base; without it the
/// base is the job's use case.
pub fn resolve_rules(
    config: &Config,
    job: &Job,
    job_base: &Path,
    tools: &ToolRegistry,
) -> Result<Prepared> {
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
    let enabled: Vec<String> = tools.manifests().map(|m| m.name.clone()).collect();
    let merged = obligations::merge(&MergeInput {
        base: &base,
        profile: profile.as_deref(),
        extra: &extra,
        extra_names: &names,
        job: Some(job),
        enabled_tools: &enabled,
    })?;
    let resolved = obligations::resolve(&merged, job)?;
    let images: Vec<(String, crate::job::Role)> =
        job.images.iter().map(|i| (i.id.clone(), i.role)).collect();
    let checklist = Checklist::build(&resolved, &images)?;
    let system_prompt = render::system_prompt(&resolved, job, tools.manifests());
    let mut hints = BTreeMap::new();
    for (tool, table) in &merged.hints {
        hints.insert(tool.clone(), serde_json::to_value(table)?);
    }
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
        hints,
    })
}
