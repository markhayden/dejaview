//! The run state machine (SPEC "Run state machine"): resolve, prepare, reserve, tool turns, the
//! final turn, aggregate, record. A decision is one or two independent runs combined (`decision`).
//! Every step is journaled as a typed event (`events`); the report is derived from the journal
//! (`report::Report::rebuild`), so what the run holds in memory is only what the loop needs.
//! The cache is consulted once per decision, before the first run (`cache`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use schemars::JsonSchema;

use crate::aggregate::{self, Verdict};
use crate::artifacts::{ArtifactRef, ArtifactStore};
use crate::cache::{Cache, CacheStatus, KeyInput};
use crate::checklist::Checklist;
use crate::config::Config;
use crate::decision::Decision;
use crate::events::{self, Event};
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
use crate::report::{DecisionHeader, ItemRow, REPORT_VERSION, Report};
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
    pub cache: CacheStatus,
}

/// Exit code for a decision that ended in an error under the default policy.
pub const EXIT_ERROR: i32 = 3;

/// The reserved tool the model submits with.
pub const SUBMIT: &str = "submit_verdict";

/// What a decision runs with: the adapter, the tools, and the optional cross-cutting services.
pub struct Services<'a> {
    pub provider: &'a dyn Provider,
    pub tools: &'a ToolRegistry,
    /// The evaluation's shared ledger, reserved against in addition to the per-job cap.
    pub shared_ledger: Option<&'a dyn Ledger>,
    /// Cooperative cancellation (SIGINT, SIGTERM): the run appends `cancelled` and ends.
    pub cancel: Option<&'a Cancel>,
    /// An evaluation run: never reads the cache (its records must be fresh).
    pub evaluation: bool,
    /// Test knob: fail the first run's journal append at this sequence number, as if the process
    /// had died between an artifact write and its event.
    pub fail_journal_at: Option<u64>,
}

impl<'a> Services<'a> {
    pub fn new(provider: &'a dyn Provider, tools: &'a ToolRegistry) -> Services<'a> {
        Services {
            provider,
            tools,
            shared_ledger: None,
            cancel: None,
            evaluation: false,
            fail_journal_at: None,
        }
    }

    pub fn with_shared_ledger(mut self, ledger: Option<&'a dyn Ledger>) -> Services<'a> {
        self.shared_ledger = ledger;
        self
    }

    pub fn with_cancel(mut self, cancel: &'a Cancel) -> Services<'a> {
        self.cancel = Some(cancel);
        self
    }
}

/// A cancellation handle: `cancel()` from a signal handler, `cancelled()` awaited by the run.
/// Clones share the signal.
#[derive(Debug, Clone)]
pub struct Cancel {
    tx: tokio::sync::watch::Sender<bool>,
}

impl Default for Cancel {
    fn default() -> Self {
        Cancel {
            tx: tokio::sync::watch::Sender::new(false),
        }
    }
}

impl Cancel {
    pub fn new() -> Cancel {
        Cancel::default()
    }

    pub fn cancel(&self) {
        self.tx.send_replace(true);
    }

    pub fn is_cancelled(&self) -> bool {
        *self.tx.borrow()
    }

    /// Resolves once `cancel` has been called.
    pub async fn cancelled(&self) {
        let mut rx = self.tx.subscribe();
        while !*rx.borrow_and_update() {
            if rx.changed().await.is_err() {
                return;
            }
        }
    }
}

/// Runs one decision (`budget.decision_runs` independent runs) under `out_dir/<decision dir>/`:
/// the header first, each run's journal under `run-<n>/`, then the report derived from them. The
/// decision envelope is reserved against a per-job ledger capped at `budget.spend_cap_usd` and,
/// when given, the services' shared ledger (the evaluation's).
pub async fn check(
    config: &Config,
    job: &Job,
    job_base: &Path,
    services: &Services<'_>,
    out_dir: &Path,
    harness_version: &str,
) -> Result<Outcome> {
    let provider = services.provider;
    let tools = services.tools;
    let shared = services.shared_ledger;
    if !(1..=2).contains(&config.budget.decision_runs) {
        return Err(Error::Config("decision_runs must be 1 or 2".into()));
    }
    let dir = new_decision_dir(out_dir)?;
    let setup = setup(config, job, job_base, tools);

    // The cache, once per decision: a hit returns the whole cached decision before any request.
    let cache = Cache::from_config(&config.cache)?;
    let mut cache_status = CacheStatus::off();
    if let (Ok(s), Some(cache)) = (&setup, &cache) {
        let key = KeyInput::gather(
            config,
            job,
            &s.images,
            &s.prepared,
            tools,
            provider,
            harness_version,
        )
        .key()?;
        cache_status = CacheStatus {
            status: if services.evaluation {
                "bypass"
            } else {
                "miss"
            }
            .into(),
            key: Some(key.clone()),
            source: None,
        };
        if config.cache.read
            && !services.evaluation
            && let Some(entry) = cache.lookup(&key)?
        {
            cache_status.status = "hit".into();
            cache_status.source = Some(entry.decision_dir.clone());
        }
    }
    let header = DecisionHeader {
        report_version: REPORT_VERSION,
        harness_version: harness_version.to_string(),
        use_case: job.use_case.clone(),
        job: job.summary(),
        config: config.redacted_json(),
        rules: setup.as_ref().ok().map(|s| s.prepared.summary.clone()),
        prompt_version: render::PROMPT_VERSION.to_string(),
        provider: provider.id().to_string(),
        tools: tools
            .manifests()
            .map(|m| format!("{}@{}", m.name, m.version))
            .collect(),
        decision_runs: config.budget.decision_runs,
        policy: config.policy.clone(),
        cap_usd: config.budget.spend_cap_usd,
        envelope: None,
        cache: cache_status.clone(),
        recipient: provider.recipient(),
    };
    if cache_status.is_hit() {
        header.write(&dir)?;
        let report = Report::rebuild(&dir)?;
        report.write(&dir)?;
        return Ok(outcome(dir, report));
    }

    let job_ledger = CapLedger::new(config.budget.spend_cap_usd);
    let envelope = match (&setup, provider.pricing()) {
        (Ok(s), Some(pricing)) => Some(ledger::plan(
            config,
            &plan_input(&s.prepared, job, tools),
            pricing,
        )),
        (Ok(_), None) => Some(Err(Error::Budget(
            "the provider cannot bound request costs; refusing to run".into(),
        ))),
        (Err(_), _) => None,
    };
    DecisionHeader {
        envelope: envelope.as_ref().and_then(|e| e.as_ref().ok().cloned()),
        ..header
    }
    .write(&dir)?;

    let mut money = Money {
        envelope: &envelope,
        pricing: provider.pricing(),
        job_ledger: &job_ledger,
        shared,
        account: None,
    };

    let mut runs_result = Ok(());
    for n in 1..=config.budget.decision_runs {
        let run_dir = dir.join(format!("run-{n}"));
        if let Err(e) = std::fs::create_dir(&run_dir) {
            runs_result = Err(e.into());
            break;
        }
        runs_result = run_once(config, services, &setup, &run_dir, n, &mut money).await;
        if runs_result.is_err() || services.cancel.is_some_and(Cancel::is_cancelled) {
            // A cancelled decision stops here; the report treats the missing runs as an error.
            break;
        }
    }
    // The reservation is given back whatever happened, so a shared ledger never leaks it.
    if let Some(account) = money.account.take() {
        let closing = account.close();
        job_ledger.settle(closing.envelope_usd, closing.spent_usd);
        if let Some(s) = shared {
            s.settle(closing.envelope_usd, closing.spent_usd);
        }
    }
    runs_result?;

    let report = Report::rebuild(&dir)?;
    report.write(&dir)?;
    // Written once, after every run, only when none errored.
    if let (Some(cache), Some(key)) = (&cache, &cache_status.key)
        && config.cache.write
        && report.verdict.is_some()
        && report.runs.iter().all(|r| r.error.is_none())
    {
        cache.store(key, &dir)?;
    }
    Ok(outcome(dir, report))
}

fn outcome(dir: PathBuf, report: Report) -> Outcome {
    Outcome {
        dir,
        error: report
            .runs
            .iter()
            .find_map(|r| r.error.as_ref().map(|e| e.message.clone())),
        exit_code: report.exit_code,
        decision: report.decision,
        cache: report.cache,
    }
}

/// What every run of a decision shares: the resolved rules and the decoded images.
pub struct Setup {
    pub prepared: Prepared,
    pub images: Vec<CanonicalImage>,
}

/// The decision envelope a run of this job would reserve (SPEC "Cost ledger"), without running:
/// the same setup and plan `check` uses, so an estimate equals the ledger's reservation.
pub fn envelope(
    config: &Config,
    job: &Job,
    job_base: &Path,
    tools: &ToolRegistry,
    pricing: &dyn Pricing,
) -> Result<ledger::Envelope> {
    let s = setup(config, job, job_base, tools)?;
    ledger::plan(config, &plan_input(&s.prepared, job, tools), pricing)
}

/// Resolves the rules and decodes the images once per decision.
pub fn setup(config: &Config, job: &Job, job_base: &Path, tools: &ToolRegistry) -> Result<Setup> {
    let prepared = resolve_rules(config, job, job_base, tools)?;
    let bounds = config.images.decode_bounds();
    let images = job
        .images
        .iter()
        .map(|spec| image::load(spec, job_base, &bounds))
        .collect::<Result<Vec<_>>>()?;
    Ok(Setup { prepared, images })
}

/// One run of the state machine, recorded in its journal. Never fails for run-level reasons:
/// those become the run's terminal event; only I/O on the journal propagates.
async fn run_once(
    config: &Config,
    services: &Services<'_>,
    setup: &Result<Setup>,
    run_dir: &Path,
    n: u32,
    money: &mut Money<'_>,
) -> Result<()> {
    let mut journal = Journal::create(run_dir)?;
    if let Some(seq) = services.fail_journal_at
        && n == 1
    {
        journal.fail_at(seq);
    }
    let mut state = RunState::new(run_dir, n, &config.secrets)?;
    let result = match setup {
        Ok(s) => drive(config, services, s, &mut journal, &mut state, money).await,
        // The rules or an image failed to resolve; the class is the error's own.
        Err(Error::Image(m)) => Err(Error::Image(m.clone())),
        Err(e) => Err(Error::Config(e.to_string())),
    };
    match result {
        Ok(verdict) => journal.append(&Event::Verdict(events::VerdictReached { verdict })),
        Err(e) => {
            // A cancellation ends the journal in `cancelled` with the state reached.
            let terminal = events::Terminal {
                class: e.class().into(),
                message: e.to_string(),
                state: journal.last_event().map(str::to_string),
            };
            journal.append(&if matches!(e, Error::Cancelled(_)) {
                Event::Cancelled(terminal)
            } else {
                Event::Error(terminal)
            })
        }
    }
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
                // Echoed thinking is billed as input like any other block.
                ContentBlock::Opaque { block, .. } => bytes += block.to_string().len() as u64,
            }
        }
    }
    RequestShape {
        input_tokens: ledger::estimated_tokens(bytes),
        output_tokens: u64::from(request.max_output_tokens),
        images,
    }
}

/// What the loop needs between steps; everything reportable goes to the journal.
struct RunState {
    run_dir: PathBuf,
    run_id: String,
    artifacts: ArtifactStore,
    images_sent: BTreeMap<String, ArtifactRef>,
    images: Vec<CanonicalImage>,
    /// The bytes the provider sends per content-block id (job image ids and frame ids).
    prepared_images: BTreeMap<String, image::Prepared>,
    instances: Vec<Instance>,
    checklist: Checklist,
    registry: Registry,
    /// Lossless pixels per frame id, written under the run directory for tools.
    frame_files: BTreeMap<String, PathBuf>,
    completed_calls: Vec<String>,
    /// Requests sent so far; the next turn's number is one more.
    turns: u32,
    repairs_used: u32,
    next_call: u32,
}

impl RunState {
    fn new(run_dir: &Path, n: u32, secrets: &[String]) -> Result<RunState> {
        Ok(RunState {
            run_dir: run_dir.to_path_buf(),
            run_id: format!("run-{n}"),
            artifacts: ArtifactStore::create(run_dir, secrets)?,
            images_sent: BTreeMap::new(),
            images: Vec::new(),
            prepared_images: BTreeMap::new(),
            instances: Vec::new(),
            checklist: Checklist::default(),
            registry: Registry::new(),
            frame_files: BTreeMap::new(),
            completed_calls: Vec::new(),
            turns: 0,
            repairs_used: 0,
            next_call: 1,
        })
    }

    /// Prepares an image for the provider, stores the bytes as sent under `images_sent` (by frame
    /// id) and keeps them for the requests (by the content-block id).
    fn prepare_for_provider(
        &mut self,
        provider: &dyn Provider,
        block_id: &str,
        frame_id: &str,
        pixels: &::image::DynamicImage,
    ) -> Result<ArtifactRef> {
        let prepared = provider.prepare_image(block_id, pixels)?;
        let r = self.artifacts.put(&prepared.bytes)?;
        self.images_sent.insert(frame_id.to_string(), r.clone());
        self.prepared_images.insert(block_id.to_string(), prepared);
        Ok(r)
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
        crate::artifacts::make_private(&path)?;
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

/// A call the model made that the harness did not execute (call budget, final turn).
fn not_executed(call: &ToolCall, reason: &str) -> Event {
    Event::ToolCalled(events::ToolCalled {
        call_id: None,
        provider_call_id: call.id.clone(),
        tool: call.name.clone(),
        version: None,
        arguments: None,
        executed: false,
        reason: Some(reason.into()),
        invalid_args: None,
        status: None,
        duration_ms: None,
    })
}

async fn drive(
    config: &Config,
    services: &Services<'_>,
    setup: &Setup,
    journal: &mut Journal,
    state: &mut RunState,
    money: &mut Money<'_>,
) -> Result<Verdict> {
    let provider = services.provider;
    let tools = services.tools;
    let prepared = &setup.prepared;
    // Resolve: the shared images and rules become this run's state.
    state.images = setup.images.clone();
    state.instances = prepared.resolved.instances.clone();
    state.checklist = prepared.checklist.clone();
    for img in &state.images {
        state
            .registry
            .add_raw(&img.id, img.width, img.height, img.projection);
    }
    // Every image the model will see is prepared for the provider and stored as sent before the
    // event that names it.
    let job_images: Vec<(String, ::image::DynamicImage)> = state
        .images
        .iter()
        .map(|i| i.dynamic().map(|d| (i.id.clone(), d)))
        .collect::<Result<_>>()?;
    for (id, pixels) in &job_images {
        let frame = format!("raw:{id}");
        state.prepare_for_provider(provider, id, &frame, pixels)?;
        // The lossless pixels tools read, written once up front.
        state.frame_file(&frame)?;
    }
    journal.append(&Event::Resolved(events::Resolved {
        images: state.images.clone(),
        images_sent: state.images_sent.clone(),
        instances: state.instances.clone(),
        checklist: state.checklist.clone(),
        tools: tools.manifests().map(|m| m.name.clone()).collect(),
    }))?;
    journal.append(&Event::Prepared(events::Prepared {
        prep_version: provider.prep_version().to_string(),
        images: state.images.len(),
    }))?;

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
    journal.append(&Event::Reserved(events::Reserved {
        envelope: envelope.clone(),
        cap_usd: config.budget.spend_cap_usd,
        shared_cap_usd: money.shared.map(Ledger::cap),
    }))?;
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
                    .map(|l| format!(", {}", render::clean(l)))
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
            services.cancel,
        )
        .await?;
        allowance = config.provider.max_output_tokens;

        match classify(&response.stop_reason, &response.tool_calls) {
            Class::Submission(call) => {
                history.push(assistant_message(&response));
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
                history.push(assistant_message(&response));
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
                history.push(assistant_message(&response));
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
                        journal.append(&not_executed(c, "call budget"))?;
                    }
                    history.push(Message {
                        role: MessageRole::User,
                        content: results,
                    });
                    break;
                }
                let mut results: Vec<ContentBlock> = Vec::new();
                for c in calls {
                    results.push(
                        execute_call(
                            config,
                            provider,
                            tools,
                            prepared,
                            journal,
                            state,
                            c,
                            services.cancel,
                        )
                        .await?,
                    );
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
        role: MessageRole::System,
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
            services.cancel,
        )
        .await?;
        allowance = config.provider.max_output_tokens;
        match classify(&response.stop_reason, &response.tool_calls) {
            Class::Submission(call) => {
                history.push(assistant_message(&response));
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
                history.push(assistant_message(&response));
                for c in calls {
                    journal.append(&not_executed(c, "final_turn"))?;
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
                    return Err(Error::ModelOutput(format!(
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
    if state.repairs_used >= config.budget.repairs {
        return Ok(false);
    }
    state.repairs_used += 1;
    journal.append(&Event::Repair(events::Repair {
        form: form.into(),
        reason: reason.into(),
        after_turn: state.turns,
        repairs_used: state.repairs_used,
    }))?;
    Ok(true)
}

/// Journals a non-replayable response; it never enters provider history.
fn exclude(
    journal: &mut Journal,
    state: &RunState,
    response: &TurnResponse,
    reason: Excluded,
) -> Result<()> {
    journal.append(&Event::TurnExcluded(events::TurnExcluded {
        turn: state.turns,
        stop_reason: response.stop_reason.clone(),
        reason: reason.name().into(),
        text_blocks: response.text.len(),
        tool_calls: response.tool_calls.len(),
    }))
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

/// The assistant message as the provider must see it again: the adapter's ordered content when it
/// gave one (opaque blocks included), otherwise the typed view.
fn assistant_message(response: &TurnResponse) -> Message {
    let content: Vec<ContentBlock> = if response.content.is_empty() {
        let mut content: Vec<ContentBlock> = response
            .text
            .iter()
            .map(|t| ContentBlock::Text { text: t.clone() })
            .collect();
        for c in &response.tool_calls {
            content.push(ContentBlock::ToolUse {
                id: c.id.clone(),
                name: c.name.clone(),
                arguments: c.arguments.clone(),
            });
        }
        content
    } else {
        response.content.clone()
    };
    Message {
        role: MessageRole::Assistant,
        content,
    }
}

/// Sends one request with transport retries, journaling the request, every retry and the response.
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
    cancel: Option<&Cancel>,
) -> Result<TurnResponse> {
    let turn = state.turns + 1;
    if let Some(c) = cancel
        && c.is_cancelled()
    {
        return Err(Error::Cancelled(format!("before turn {turn}")));
    }
    let request = TurnRequest {
        system: &prepared.system_prompt,
        history,
        tools: definitions,
        max_output_tokens,
        images: &state.prepared_images,
    };
    let bound_usd = pricing.bound(&shape_of(&request))?;
    // The model-visible request body, with images by frame id (their bytes are artifacts of their
    // own), redacted and bounded.
    let request_ref = state
        .artifacts
        .put_body(&serde_json::to_string(&serde_json::json!({
            "system": request.system,
            "history": request.history,
            "tools": request.tools,
            "max_output_tokens": request.max_output_tokens,
        }))?)?;
    journal.append(&Event::TurnSent(events::TurnSent {
        turn,
        tools: definitions.iter().map(|t| t.name.clone()).collect(),
        history_messages: history.len(),
        max_output_tokens,
        bound_usd,
        request: request_ref,
    }))?;
    state.turns = turn;
    // A failed attempt's usage is unknown: its hold is charged at the bound. The retry that
    // follows draws on the envelope's retry reserve; once the balance cannot cover another
    // attempt the retry is refused instead of overrunning the envelope.
    let mut charged_usd = 0.0;
    let mut on_retry = |a: &retry::Attempt| -> Result<()> {
        let hold = account.hold(bound_usd);
        account.charge(hold);
        charged_usd += bound_usd;
        journal.append(&Event::Retry(events::Retry {
            turn,
            attempt: a.attempt,
            status: a.status,
            delay_ms: a.delay.as_millis() as u64,
            error: a.message.clone(),
            charged_usd: bound_usd,
        }))?;
        if !account.covers(bound_usd) {
            return Err(Error::Budget(format!(
                "turn {turn}: the envelope cannot cover another attempt (${:.4} unallocated, bound ${bound_usd:.4}) after {} failed attempt(s); the retry reserve is spent",
                account.unallocated(),
                a.attempt
            )));
        }
        Ok(())
    };
    let backoff = retry::Backoff::default();
    let mut jitter = rand::random::<f64>;
    let send = retry::send_with_retries(
        provider,
        request,
        config.budget.retries,
        &backoff,
        &retry::TokioSleeper,
        &mut jitter,
        &mut on_retry,
    );
    let sent = match cancel {
        Some(c) => {
            tokio::select! {
                r = send => r,
                _ = c.cancelled() => Err(Error::Cancelled(format!("while waiting for turn {turn}"))),
            }
        }
        None => send.await,
    };
    let sent = match sent {
        Ok(sent) => sent,
        Err(e) => {
            // The attempt that failed was sent too: unknown usage, charged at the bound. A
            // cancellation is journaled as the terminal event with the state reached, so the
            // charge is recorded there only in the ledger.
            let hold = account.hold(bound_usd);
            account.charge(hold);
            if !matches!(e, Error::Cancelled(_)) {
                let attempts = 1 + (charged_usd / bound_usd.max(f64::MIN_POSITIVE)).round() as u32;
                journal.append(&Event::AttemptFailed(events::AttemptFailed {
                    turn,
                    attempts,
                    class: e.class().into(),
                    error: e.to_string(),
                    charged_usd: bound_usd,
                }))?;
            }
            return Err(e);
        }
    };
    let response = sent.response;
    let hold = account.hold(bound_usd);
    let settled_usd = pricing.cost(&response.usage);
    account.settle(hold, settled_usd);
    let response_ref = state
        .artifacts
        .put_body(&serde_json::to_string(&response)?)?;
    journal.append(&Event::TurnReceived(events::TurnReceived {
        turn,
        stop_reason: response.stop_reason.clone(),
        tool_calls: response.tool_calls.iter().map(|c| c.name.clone()).collect(),
        usage: response.usage,
        provider_request_id: response.provider_request_id.clone(),
        attempts: sent.attempts,
        cost_usd: settled_usd + charged_usd,
        response: response_ref,
    }))?;
    Ok(response)
}

/// Validates and executes one tool call, returning the result block for the model. A model-side
/// mistake is an error result; a tool failure or bound breach is `error: tool`. The call itself
/// runs on its own thread so a signal lands while a tool works.
#[allow(clippy::too_many_arguments)]
async fn execute_call(
    config: &Config,
    provider: &dyn Provider,
    tools: &ToolRegistry,
    prepared: &Prepared,
    journal: &mut Journal,
    state: &mut RunState,
    call: &ToolCall,
    cancel: Option<&Cancel>,
) -> Result<ContentBlock> {
    let call_id = state.next_call_id();
    let started = Instant::now();
    let mut status: Option<Status> = None;

    // Model-side mistakes become error results, never operational failures.
    macro_rules! reject {
        ($message:expr) => {{
            let message: String = $message;
            journal.append(&Event::ToolCalled(events::ToolCalled {
                call_id: Some(call_id.clone()),
                provider_call_id: call.id.clone(),
                tool: call.name.clone(),
                version: None,
                arguments: Some(call.arguments.clone()),
                executed: false,
                reason: None,
                invalid_args: Some(message.clone()),
                status,
                duration_ms: Some(started.elapsed().as_millis() as u64),
            }))?;
            return Ok(ContentBlock::ToolResult {
                tool_use_id: call.id.clone(),
                content: format!("invalid arguments: {message}"),
                is_error: true,
                images: vec![],
            });
        }};
    }

    let Some(tool) = tools.get_shared(&call.name) else {
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
        run_dir: Some(state.run_dir.clone()),
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
        let (parent, transform, width, height) = match &frame {
            crate::frames::Frame::Raw { width, height, .. } => (None, None, *width, *height),
            crate::frames::Frame::Aligned {
                parent,
                transform_id,
                width,
                height,
                ..
            } => (
                Some(parent.clone()),
                Some(serde_json::json!({ "id": transform_id })),
                *width,
                *height,
            ),
            crate::frames::Frame::Crop {
                parent,
                mapping,
                width,
                height,
                ..
            } => (
                Some(parent.clone()),
                Some(serde_json::to_value(mapping)?),
                *width,
                *height,
            ),
        };
        let media_type = if path.extension().is_some_and(|e| e == "jpg") {
            "image/jpeg"
        } else {
            "image/png"
        };
        ctx.frames.insert(
            id.to_string(),
            FrameHandle {
                root,
                projection,
                parent,
                transform,
                pixels: ImageHandle {
                    media_type: media_type.into(),
                    width,
                    height,
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
    journal.append(&Event::ToolCalled(events::ToolCalled {
        call_id: Some(call_id.clone()),
        provider_call_id: call.id.clone(),
        tool: manifest.name.clone(),
        version: Some(manifest.version.clone()),
        arguments: Some(call.arguments.clone()),
        executed: true,
        reason: None,
        invalid_args: None,
        status: None,
        duration_ms: None,
    }))?;

    // The tool runs on its own thread; the run keeps listening for a cancellation meanwhile.
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let tool = std::sync::Arc::clone(&tool);
        let args = call.arguments.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(tool.call(&args, &ctx));
        });
    }
    let outcome = match cancel {
        Some(c) => {
            tokio::select! {
                r = rx => r,
                _ = c.cancelled() => {
                    return Err(Error::Cancelled(format!("during tool call {call_id}")));
                }
            }
        }
        None => rx.await,
    };
    let mut envelope = outcome
        .map_err(|_| Error::Tool(format!("{}: the tool's thread died", manifest.name)))??;
    let duration_ms = started.elapsed().as_millis() as u64;
    // The structured result in full, before anything is dropped or presented.
    let envelope_ref = state
        .artifacts
        .put_body(&serde_json::to_string(&envelope)?)?;
    if envelope.status == Status::InvalidArgs {
        status = Some(Status::InvalidArgs);
        let msg = envelope.summary.clone();
        reject!(msg);
    }
    envelope.check_bounds(&manifest.returns)?;

    // Returned images become crop frames with their pixels stored for later calls. Their ids are
    // the harness's to assign (`crop:<call id>:<n>`), never an existing frame, and always with
    // their bytes.
    for (n, img) in envelope.images.iter().enumerate() {
        let expected = format!("crop:{call_id}:{n}");
        if img.frame != expected {
            return Err(Error::Tool(format!(
                "{}: returned image {n} is named {:?}; it must be {expected:?}",
                manifest.name, img.frame
            )));
        }
        if state.registry.get(&img.frame).is_some() {
            return Err(Error::Tool(format!(
                "{}: returned image {:?} is already a frame",
                manifest.name, img.frame
            )));
        }
        if state.registry.get(&img.parent).is_none() {
            return Err(Error::Tool(format!(
                "{}: returned image parent {:?} is not a frame",
                manifest.name, img.parent
            )));
        }
        let Some(b64) = &img.base64 else {
            return Err(Error::Tool(format!(
                "{}: returned image {:?} carries no bytes",
                manifest.name, img.frame
            )));
        };
        let bytes = image::base64_decode(b64)
            .map_err(|e| Error::Tool(format!("{}: returned image: {e}", manifest.name)))?;
        state.registry.add_crop(
            &img.frame,
            &img.parent,
            img.mapping.clone(),
            img.width,
            img.height,
        );
        let dir = state.run_dir.join("frames");
        std::fs::create_dir_all(&dir)?;
        let ext = if img.media_type == "image/jpeg" {
            "jpg"
        } else {
            "png"
        };
        let path = dir.join(format!("{}.{ext}", sanitise(&img.frame)));
        crate::artifacts::write_private(&path, &bytes)?;
        state.frame_files.insert(img.frame.clone(), path);
        let decoded = ::image::load_from_memory(&bytes).map_err(|e| {
            Error::Tool(format!(
                "{}: returned image {} cannot be decoded: {e}",
                manifest.name, img.frame
            ))
        })?;
        state.prepare_for_provider(provider, &img.frame, &img.frame, &decoded)?;
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
        journal.append(&Event::EvidenceDropped(events::EvidenceDropped {
            call_id: d.call_id.clone(),
            region_id: d.region_id.clone(),
            reason: d.reason.clone(),
        }))?;
    }
    if matches!(
        envelope.status,
        Status::Ok | Status::MeasuredNothing | Status::NotApplicable
    ) {
        state.completed_calls.push(call_id.clone());
    }
    journal.append(&Event::ToolReturned(events::ToolReturned {
        call_id: call_id.clone(),
        status: envelope.status,
        regions: envelope.regions.len(),
        images: envelope
            .images
            .iter()
            .map(|i| events::ReturnedImage {
                frame: i.frame.clone(),
                artifact: state.images_sent.get(&i.frame).cloned(),
                parent: i.parent.clone(),
                mapping: i.mapping.clone(),
                width: i.width,
                height: i.height,
            })
            .collect(),
        added: ingest.added.clone(),
        checklist: state.checklist.clone(),
        duration_ms,
        envelope: envelope_ref,
    }))?;

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

/// Validates the submission, journals it with its aggregation rows, and returns the verdict.
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

    let mut rows = Vec::new();
    let mut items = Vec::new();
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
        items.push(ItemRow {
            id: item.id.clone(),
            disposition: item.disposition,
            confidence: item.confidence,
            reason: item.reason.clone(),
            allowed_by: item.allowed_by.clone(),
            location: item.location.clone(),
            row,
        });
    }
    let mut finding_rows = Vec::new();
    let mut finding_entries = Vec::new();
    for f in &sub.findings {
        let mapped = state.registry.to_root(&f.location).ok();
        let root_image = mapped.as_ref().and_then(|m| {
            state
                .registry
                .get(&m.location.frame)
                .map(|fr| fr.root().to_string())
        });
        finding_entries.push(crate::report::FindingEntry {
            obligation_ids: f.obligation_ids.clone(),
            disposition: f.disposition,
            confidence: f.confidence,
            reason: f.reason.clone(),
            allowed_by: f.allowed_by.clone(),
            location: f.location.clone(),
            root: match (&mapped, &root_image) {
                (Some(m), Some(image)) => Some(crate::report::RootLocation {
                    image: image.clone(),
                    frame: m.location.frame.clone(),
                    kind: m.location.kind,
                    bbox: m.location.bbox,
                    polygon: m.location.polygon.clone(),
                    partial: m.partial,
                }),
                _ => None,
            },
        });
        let d: aggregate::Disposition = f.disposition.into();
        if d == aggregate::Disposition::Observed {
            continue;
        }
        let root = root_image.clone();
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
        finding_rows.push(row);
    }
    journal.append(&Event::Submission(events::Submission {
        provider_call_id: call.id.clone(),
        items,
        findings: sub.findings.len(),
        finding_rows,
        finding_entries,
    }))?;
    Ok(aggregate::verdict(&rows))
}

fn new_decision_dir(out_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let pid = std::process::id();
    let mut suffix = 0u32;
    loop {
        let name = if suffix == 0 {
            format!("decision-{unix_ms}-{pid}")
        } else {
            format!("decision-{unix_ms}-{pid}-{suffix}")
        };
        let dir = out_dir.join(name);
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && suffix < 1000 => {
                suffix += 1;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// What the rules resolved to, for the report and the cache key.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
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
