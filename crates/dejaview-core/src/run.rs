//! The run state machine (SPEC "Run state machine"): resolve, prepare, reserve, tool turns, the
//! final turn, aggregate, record. A decision is one or two independent runs combined (`decision`).
//! Task 1.7a: tool turns with batch validation, the call budget, result ingestion. Task 1.7b: the
//! final turn's refuse mechanism and every terminal error class. Task 1.8: repairs and retries.
//! Tasks 1.9 to 1.11: ledger, journal artifacts, cache.

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
use crate::obligations::{self, Instance, MergeInput, Resolved, RulesFile};
use crate::provider::{
    ContentBlock, Message, MessageRole, Provider, StopReason, ToolCall, ToolDefinition,
    TurnRequest, TurnResponse,
};
use crate::render;
use crate::report::{
    ErrorEntry, InstanceEntry, ItemRow, REPORT_VERSION, Report, RunReport, ToolCallEntry, TurnEntry,
};
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
/// `out_dir/<decision dir>/`, with each run's journal under `run-<n>/`.
pub async fn check(
    config: &Config,
    job: &Job,
    job_base: &Path,
    provider: &dyn Provider,
    tools: &ToolRegistry,
    out_dir: &Path,
    harness_version: &str,
) -> Result<Outcome> {
    let dir = new_decision_dir(out_dir)?;
    let prepared = resolve_rules(config, job, job_base, tools);

    let mut runs: Vec<RunReport> = Vec::new();
    let mut summaries: Vec<RunSummary> = Vec::new();
    let mut first_error: Option<String> = None;
    for n in 1..=config.budget.decision_runs {
        let run_dir = dir.join(format!("run-{n}"));
        std::fs::create_dir(&run_dir)?;
        let (report, summary, error) = run_once(
            config, job, job_base, provider, tools, &prepared, &run_dir, n,
        )
        .await?;
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
    // Prepare and reserve are stubs until tasks 2.3 and 1.9; the events exist so the journal's
    // shape is final.
    journal.append(
        "prepared",
        serde_json::json!({ "prep_version": "canonical", "images": state.images.len() }),
    )?;
    journal.append(
        "reserved",
        serde_json::json!({ "envelope_usd": null, "cap_usd": config.budget.spend_cap_usd }),
    )?;

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
    let mut turn: u32 = 0;
    while turn < config.budget.tool_turns {
        turn += 1;
        let response = send_turn(
            config,
            provider,
            prepared,
            &history,
            &definitions,
            journal,
            state,
            turn,
        )
        .await?;
        history.push(assistant_message(&response.text, &response.tool_calls));

        match classify(&response.stop_reason, &response.tool_calls) {
            Class::Submission(call) => return finish(prepared, state, journal, call),
            Class::MixedCalls(_) => {
                return Err(Error::ModelOutput(
                    "submit_verdict must be the only call in its batch (repairs arrive in task 1.8)".into(),
                ));
            }
            Class::NoCall => {
                return Err(Error::ModelOutput(format!(
                    "the model ended turn {turn} ({:?}) without calling a tool or submitting (repairs arrive in task 1.8)",
                    response.stop_reason
                )));
            }
            Class::Truncated => {
                return Err(Error::ModelOutput(
                    "the response was truncated (max_tokens); repairs arrive in task 1.8".into(),
                ));
            }
            Class::Batch(calls) => {
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
    // result; the single repair (task 1.8) may then ask once more, otherwise the run ends
    // `error: no_submission`.
    history.push(Message {
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: final_turn_instruction(state),
        }],
    });
    let response = send_turn(
        config,
        provider,
        prepared,
        &history,
        &definitions,
        journal,
        state,
        turn + 1,
    )
    .await?;
    history.push(assistant_message(&response.text, &response.tool_calls));
    match classify(&response.stop_reason, &response.tool_calls) {
        Class::Submission(call) => finish(prepared, state, journal, call),
        Class::Batch(calls) | Class::MixedCalls(calls) => {
            let results: Vec<ContentBlock> = calls
                .iter()
                .map(|c| ContentBlock::ToolResult {
                    tool_use_id: c.id.clone(),
                    content: format!("Not available: only {SUBMIT} is executed on the final turn. Submit your verdict now."),
                    is_error: true,
                    images: vec![],
                })
                .collect();
            for c in calls {
                journal.append(
                    "tool_called",
                    serde_json::json!({ "tool": c.name, "provider_call_id": c.id, "executed": false, "reason": "final_turn" }),
                )?;
            }
            history.push(Message {
                role: MessageRole::User,
                content: results,
            });
            Err(Error::NoSubmission(format!(
                "the final turn called {:?} instead of submitting; the repair arrives in task 1.8",
                calls.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()
            )))
        }
        Class::NoCall => Err(Error::NoSubmission(format!(
            "the final turn ended ({:?}) without a submission",
            response.stop_reason
        ))),
        Class::Truncated => Err(Error::NoSubmission(
            "the final turn's response was truncated".into(),
        )),
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

#[derive(Debug)]
enum Class<'a> {
    Submission(&'a ToolCall),
    Batch(&'a [ToolCall]),
    /// A batch that mixes submit_verdict with other calls.
    MixedCalls(&'a [ToolCall]),
    NoCall,
    Truncated,
}

fn classify<'a>(stop: &StopReason, calls: &'a [ToolCall]) -> Class<'a> {
    if *stop == StopReason::MaxTokens {
        return Class::Truncated;
    }
    let submits = calls.iter().filter(|c| c.name == SUBMIT).count();
    match (calls.len(), submits) {
        (0, _) => Class::NoCall,
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

#[allow(clippy::too_many_arguments)]
async fn send_turn(
    config: &Config,
    provider: &dyn Provider,
    prepared: &Prepared,
    history: &[Message],
    definitions: &[ToolDefinition],
    journal: &mut Journal,
    state: &mut RunState,
    turn: u32,
) -> Result<TurnResponse> {
    let request = TurnRequest {
        system: &prepared.system_prompt,
        history,
        tools: definitions,
        max_output_tokens: config.provider.max_output_tokens,
    };
    journal.append(
        "turn_sent",
        serde_json::json!({ "turn": turn, "tools": definitions.iter().map(|t| t.name.clone()).collect::<Vec<_>>(), "history_messages": history.len() }),
    )?;
    let response = provider.send(request).await?;
    journal.append(
        "turn_received",
        serde_json::json!({
            "turn": turn,
            "stop_reason": response.stop_reason,
            "tool_calls": response.tool_calls.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
            "usage": response.usage,
        }),
    )?;
    state.turns.push(TurnEntry {
        turn,
        stop_reason: response.stop_reason.clone(),
        tool_calls: response.tool_calls.iter().map(|c| c.name.clone()).collect(),
        usage: response.usage,
        provider_request_id: response.provider_request_id.clone(),
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
