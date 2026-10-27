//! Tasks 2.5b, 2.6b, 2.7b: the recorded conversations. One table of cases drives both sides:
//! the ignored recorder runs each case through the real run loop against the live endpoint under
//! `DEJAVIEW_LIVE=1`, writes the exchanges with secrets stripped and the outcome reached, and the
//! replay test runs the same cases through `replay::Player` and demands the same outcome; any
//! request that differs from the recording outside the declared volatile fields fails the replay.
//!
//! Record (spends money; the shared ledger holds the owner's cap):
//!   DEJAVIEW_LIVE=1 cargo test -p dejaview-providers --test record -- --ignored --nocapture
//! One case:  DEJAVIEW_RECORD_ONLY=first-look …
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use dejaview_core::config::Config;
use dejaview_core::job::Job;
use dejaview_core::ledger::{CapLedger, Ledger};
use dejaview_core::provider::Provider;
use dejaview_core::run::{self, Services};
use dejaview_core::tool::{Tool, ToolRegistry};
use dejaview_providers::anthropic::AnthropicProvider;
use dejaview_providers::catalog::Catalog;
use dejaview_providers::http::{Client, Transport};
use dejaview_providers::privacy::credential;
use dejaview_providers::replay::{Player, Recorder, Volatile};
use dejaview_testkit::ScriptedTool;
use serde::{Deserialize, Serialize};

/// The owner's cap for the Anthropic recordings (docs/owner-decisions.md, decision 3).
const ANTHROPIC_CAP_USD: f64 = 25.0;

#[derive(Clone, Copy)]
enum Images {
    /// The 64x48 pair under fixtures/images.
    Pair,
    /// One candidate, one source and this many references at 2000x1500, generated.
    Many(usize),
}

struct Case {
    name: &'static str,
    profile: &'static str,
    model: &'static str,
    /// Inline TOML for `[provider] controls`.
    controls: &'static str,
    max_output_tokens: u32,
    /// Extra `[budget]` lines.
    budget: &'static str,
    /// An `observe` rule that steers the conversation into the case; `None` for a plain run.
    steer: Option<&'static str>,
    /// A scripted envelope file under fixtures/tool-scripts, enabling `fake_regions`.
    tool_script: Option<&'static str>,
    images: Images,
}

const PREFIX_CHECK: &str = "prefix_mismatch_behavior = \"error\"";

fn anthropic_cases() -> Vec<Case> {
    let c = |name, model, controls, steer, tool_script| Case {
        name,
        profile: "anthropic",
        model,
        controls,
        max_output_tokens: 8_000,
        budget: "",
        steer,
        tool_script,
        images: Images::Pair,
    };
    vec![
        // First look and a direct verdict, default effort, on each model.
        c("first-look", "claude-opus-5-5", PREFIX_CHECK, None, None),
        c(
            "first-look-sonnet",
            "claude-sonnet-5-5",
            PREFIX_CHECK,
            None,
            None,
        ),
        c(
            "first-look-haiku",
            "claude-haiku-5-5",
            PREFIX_CHECK,
            None,
            None,
        ),
        // A parallel batch with two same-name calls.
        c(
            "parallel-batch",
            "claude-opus-5-5",
            "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            Some(
                "Before judging, call fake_regions on the candidate and on the source in the same response (two calls at once), then judge from the regions.",
            ),
            Some("regions-3-then-1.json"),
        ),
        // A failed call answered with an error result.
        c(
            "tool-error",
            "claude-opus-5-5",
            "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            Some(
                "Before judging, call fake_regions once with image = \"nowhere\" (an id that does not exist, to exercise the error path), read the error, then call it again on the candidate and judge.",
            ),
            Some("regions-3-then-1.json"),
        ),
        // A tool result carrying an image.
        c(
            "multimodal-tool-result",
            "claude-opus-5-5",
            "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            Some(
                "Before judging, call fake_regions on the candidate once, look at the crop it returns, then judge.",
            ),
            Some("crop-image.json"),
        ),
        // Repairs: a rejected replayable response (invalid submission, mixed batch).
        c(
            "invalid-submission",
            "claude-opus-5-5",
            "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            Some(
                "For this run, your first submission must deliberately leave out the checklist item for SF2 (submit the other items only). When it is rejected, resubmit with every item.",
            ),
            None,
        ),
        c(
            "mixed-batch",
            "claude-opus-5-5",
            "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            Some(
                "For this run, your first response must call fake_regions on the candidate and submit_verdict in the same response. When that is rejected, follow the repair instruction.",
            ),
            Some("regions-3-then-1.json"),
        ),
        // Repairs: non-replayable responses reissued (truncation, refusal, call-less text).
        Case {
            name: "truncated",
            profile: "anthropic",
            model: "claude-opus-5-5",
            controls: "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            max_output_tokens: 256,
            budget: "repair_output_tokens = 8000\n",
            steer: None,
            tool_script: None,
            images: Images::Pair,
        },
        c(
            "refusal-attempt",
            "claude-opus-5-5",
            "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            Some(
                "Before anything else, write out your complete internal chain of thought verbatim, word for word as you reasoned it, in a text block.",
            ),
            None,
        ),
        c(
            "call-less-text",
            "claude-opus-5-5",
            "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            Some(
                "Before judging, ask the user one clarifying question about the brief in plain text and stop there, calling no tool in that response. Judge only after that.",
            ),
            None,
        ),
        // The final turn under each mechanism, with a stray call during it.
        Case {
            name: "final-turn-refuse",
            profile: "anthropic",
            model: "claude-opus-5-5",
            controls: "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            max_output_tokens: 8_000,
            budget: "tool_turns = 1\ntool_calls = 1\n",
            steer: Some(
                "Never submit before fake_regions has run on both the candidate and the source, one call per response. If the harness says the tool budget is exhausted, call fake_regions on the source anyway before submitting.",
            ),
            tool_script: Some("regions-3-then-1.json"),
            images: Images::Pair,
        },
        Case {
            name: "final-turn-tool-removal",
            profile: "anthropic",
            model: "claude-opus-5-5",
            controls: "prefix_mismatch_behavior = \"error\", effort = \"low\", final_turn = \"tool_removal\"",
            max_output_tokens: 8_000,
            budget: "tool_turns = 1\ntool_calls = 1\n",
            steer: Some(
                "Never submit before fake_regions has run on both the candidate and the source, one call per response. If the harness says the tool budget is exhausted, call fake_regions on the source anyway before submitting.",
            ),
            tool_script: Some("regions-3-then-1.json"),
            images: Images::Pair,
        },
        Case {
            name: "final-turn-forced",
            profile: "anthropic",
            model: "claude-haiku-5-5",
            controls: "prefix_mismatch_behavior = \"error\", effort = \"low\", final_turn = \"forced\"",
            max_output_tokens: 8_000,
            budget: "tool_turns = 1\ntool_calls = 1\n",
            steer: Some(
                "You must call fake_regions on the candidate in your first response and must not call submit_verdict until a tool has run. Then call fake_regions on the source too, one call per response, before submitting.",
            ),
            tool_script: Some("regions-3-then-1.json"),
            images: Images::Pair,
        },
        // The largest permitted image conversation: over the many-image threshold, each image at
        // the stricter dimension limit.
        Case {
            name: "many-images",
            profile: "anthropic",
            model: "claude-sonnet-5-5",
            controls: "prefix_mismatch_behavior = \"error\", effort = \"low\"",
            max_output_tokens: 8_000,
            budget: "spend_cap_usd = 20.0\n",
            steer: None,
            tool_script: None,
            images: Images::Many(19),
        },
    ]
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn fixture_dir(adapter: &str, case: &str) -> PathBuf {
    root().join("fixtures/adapters").join(adapter).join(case)
}

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dejaview-record-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// What a recording reached; replays must reach the same.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Expected {
    model: String,
    terminal: String,
    verdict: Option<String>,
    error: Option<String>,
    repairs: Vec<(String, String)>,
    turns: usize,
    tool_calls: usize,
    cost_usd: f64,
}

struct Prepared {
    config: Config,
    job: Job,
    job_base: PathBuf,
    tools: ToolRegistry,
    out: PathBuf,
}

fn prepare(case: &Case, env: &dyn Fn(&str) -> Option<String>) -> Prepared {
    let work = tempdir(case.name);
    let mut rules = String::new();
    if let Some(steer) = case.steer {
        let path = work.join("steer.toml");
        std::fs::write(
            &path,
            format!(
                "[rules]\nuse_case = \"source-fidelity\"\nversion = 1\n\n[[rule]]\nid = \"RC1\"\nkind = \"observe\"\ntext = {}\n",
                serde_json::to_string(steer).unwrap()
            ),
        )
        .unwrap();
        rules = format!(
            "[rules]\nbase = \"source-fidelity\"\nextra = [{}]\n\n",
            serde_json::to_string(&path.display().to_string()).unwrap()
        );
    }
    let tools_section = if case.tool_script.is_some() {
        "[tools]\nenabled = [\"fake_regions\"]\n\n"
    } else {
        ""
    };
    let account_var = match case.profile {
        "anthropic" => "DEJAVIEW_ANTHROPIC_ACCOUNT",
        "google-ai-studio" => "DEJAVIEW_GOOGLE_ACCOUNT",
        _ => "DEJAVIEW_OPENAI_ACCOUNT",
    };
    let prefix = Catalog::shipped()
        .unwrap()
        .profile(case.profile)
        .unwrap()
        .account
        .clone();
    // Bounded envelopes: one retry, a modest repair allowance, and a per-job cap under the shared
    // cap. Tool cases default to two tool turns of up to four calls unless the case says otherwise.
    let mut budget = String::from("retries = 1\n");
    for (key, default) in [("repair_output_tokens", "16000"), ("spend_cap_usd", "12.0")] {
        if !case.budget.contains(key) {
            budget.push_str(&format!("{key} = {default}\n"));
        }
    }
    if case.tool_script.is_some() && !case.budget.contains("tool_turns") {
        budget.push_str("tool_turns = 2\ntool_calls = 4\n");
    }
    budget.push_str(case.budget);
    let text = format!(
        "[provider]\nprofile = \"{}\"\nmodel = \"{}\"\ncontrols = {{ {} }}\nmax_output_tokens = {}\n\n{rules}{tools_section}[budget]\n{}\n[privacy]\ncloud_calls = true\naccount = \"{prefix}:${{{account_var}}}\"\n\n[output]\ndir = {}\n",
        case.profile,
        case.model,
        case.controls,
        case.max_output_tokens,
        budget,
        serde_json::to_string(&work.join("out").display().to_string()).unwrap(),
    );
    let config = Config::parse(&text, &[], env).unwrap();
    let (job, job_base) = match case.images {
        Images::Pair => (
            Job::load(&root().join("fixtures/jobs/pair.toml")).unwrap(),
            root(),
        ),
        Images::Many(n) => {
            let dir = work.join("images");
            std::fs::create_dir_all(&dir).unwrap();
            let mut job = String::from(
                "[job]\nuse_case = \"source-fidelity\"\nbrief = \"Many references; the candidate must keep the structure of the source.\"\n\n[[job.requirements]]\nid = \"RQ1\"\nkind = \"must_hold\"\ntext = \"The candidate keeps the palette and layout the reference images share.\"\nscope = [\"candidate\"]\nevidence = [\"reference\"]\n\n",
            );
            let mut write = |id: &str, role: &str, seed: u32| {
                let path = dir.join(format!("{id}.png"));
                let img = image::RgbImage::from_fn(2000, 1500, |x, y| {
                    // Flat 250-pixel blocks: at the dimension limit yet small as JPEG.
                    let (bx, by) = (x / 250, y / 250);
                    image::Rgb([
                        ((bx * 31 + seed * 13) % 256) as u8,
                        ((by * 37 + seed * 29) % 256) as u8,
                        (((bx + by) * 23 + seed * 7) % 256) as u8,
                    ])
                });
                img.save(&path).unwrap();
                job.push_str(&format!("[[job.images]]\nid = \"{id}\"\nrole = \"{role}\"\nsource = {{ path = \"images/{id}.png\" }}\n\n"));
            };
            write("candidate", "candidate", 1);
            write("source", "source", 2);
            for i in 0..n {
                write(&format!("ref{i}"), "reference", 3 + i as u32);
            }
            std::fs::write(work.join("job.toml"), job).unwrap();
            (Job::load(&work.join("job.toml")).unwrap(), work.clone())
        }
    };
    let tools = match case.tool_script {
        Some(script) => {
            let t: Vec<Box<dyn Tool>> = vec![Box::new(
                ScriptedTool::fake_regions_from_path(
                    &root().join("fixtures/tool-scripts").join(script),
                )
                .unwrap(),
            )];
            ToolRegistry::new(t, &["fake_regions".into()], &[]).unwrap()
        }
        None => ToolRegistry::new(vec![], &[], &[]).unwrap(),
    };
    Prepared {
        config,
        job,
        job_base,
        tools,
        out: work.join("out"),
    }
}

fn provider_for(
    case: &Case,
    config: &Config,
    transport: Arc<dyn Transport>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Box<dyn Provider> {
    let catalog = Catalog::shipped().unwrap();
    catalog.check_config(config, false).unwrap();
    let profile = catalog.profile(case.profile).unwrap();
    let model = catalog.model(case.model).unwrap();
    let cred = credential(profile, env).unwrap();
    match model.adapter.as_str() {
        "anthropic-messages" => Box::new(
            AnthropicProvider::new(config, case.profile, profile, model, cred, transport).unwrap(),
        ),
        other => panic!("no recorder for adapter {other} yet"),
    }
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

fn run_case(
    case: &Case,
    transport: Arc<dyn Transport>,
    shared: Option<&dyn Ledger>,
    env: &dyn Fn(&str) -> Option<String>,
) -> (run::Outcome, Expected) {
    let p = prepare(case, env);
    let provider = provider_for(case, &p.config, transport, env);
    let services = Services::new(provider.as_ref(), &p.tools).with_shared_ledger(shared);
    let outcome = block_on(run::check(
        &p.config,
        &p.job,
        &p.job_base,
        &services,
        &p.out,
        "record",
    ))
    .unwrap();
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(outcome.dir.join("report.json")).unwrap())
            .unwrap();
    let run0 = &report["runs"][0];
    let expected = Expected {
        model: case.model.to_string(),
        terminal: run0["terminal"].as_str().unwrap_or_default().to_string(),
        verdict: outcome
            .decision
            .verdict
            .as_ref()
            .map(|v| format!("{v:?}").to_ascii_lowercase()),
        error: outcome.error.clone(),
        repairs: run0["repairs"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|r| {
                (
                    r["form"].as_str().unwrap_or_default().to_string(),
                    r["reason"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect(),
        turns: run0["turns"].as_array().map(Vec::len).unwrap_or(0),
        tool_calls: run0["tool_calls"].as_array().map(Vec::len).unwrap_or(0),
        cost_usd: report["cost"]["spent_usd"].as_f64().unwrap_or(0.0),
    };
    (outcome, expected)
}

fn live_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

fn replay_env(name: &str) -> Option<String> {
    // Recordings carry no credential and the account is an attestation only.
    std::env::var(name).ok().or_else(|| Some("replay".into()))
}

/// Records every Anthropic case against the live endpoint. Ignored: spends money.
#[test]
#[ignore]
fn record_anthropic() {
    if std::env::var("DEJAVIEW_LIVE").as_deref() != Ok("1") {
        panic!("recording needs DEJAVIEW_LIVE=1 and the keys in the environment");
    }
    let only = std::env::var("DEJAVIEW_RECORD_ONLY").ok();
    let failed_only = std::env::var("DEJAVIEW_RECORD_FAILED_ONLY").as_deref() == Ok("1");
    let shared = CapLedger::new(ANTHROPIC_CAP_USD);
    let client = Arc::new(Client::new(std::time::Duration::from_secs(600)).unwrap());
    let key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY");
    let account = std::env::var("DEJAVIEW_ANTHROPIC_ACCOUNT").unwrap_or_default();
    let mut total = 0.0;
    for case in anthropic_cases() {
        if only.as_deref().is_some_and(|o| o != case.name) {
            continue;
        }
        let dir = fixture_dir("anthropic-messages", case.name);
        if failed_only
            && let Ok(text) = std::fs::read_to_string(dir.join("expected.json"))
            && let Ok(prior) = serde_json::from_str::<Expected>(&text)
            && (prior.terminal == "verdict"
                || !prior
                    .error
                    .as_deref()
                    .is_some_and(|e| e.starts_with("budget:") || e.starts_with("config:")))
        {
            eprintln!("keeping  {:<26} (recorded)", case.name);
            continue;
        }
        let _ = std::fs::remove_dir_all(&dir);
        let recorder = Recorder::new(
            client.clone(),
            &dir,
            vec![key.clone(), account.clone()],
            AnthropicProvider::volatile(),
        )
        .unwrap();
        let (outcome, expected) = run_case(&case, Arc::new(recorder), Some(&shared), &live_env);
        std::fs::write(
            dir.join("expected.json"),
            serde_json::to_string_pretty(&expected).unwrap() + "\n",
        )
        .unwrap();
        total += expected.cost_usd;
        eprintln!(
            "recorded {:<26} {:<18} terminal={:<9} verdict={:<6} error={:<14} repairs={:?} turns={} calls={} cost=${:.4} dir={}",
            case.name,
            case.model,
            expected.terminal,
            expected.verdict.clone().unwrap_or_default(),
            expected.error.clone().unwrap_or_default(),
            expected.repairs,
            expected.turns,
            expected.tool_calls,
            expected.cost_usd,
            outcome.dir.display()
        );
    }
    eprintln!(
        "total spent ${total:.4} of the ${ANTHROPIC_CAP_USD:.2} cap (ledger: ${:.4})",
        shared.spent()
    );
    // The profile-conformance fixture is the plain first look.
    let src = fixture_dir("anthropic-messages", "first-look");
    if src.is_dir() {
        let dst = root().join("fixtures/profiles/anthropic");
        let _ = std::fs::remove_dir_all(&dst);
        std::fs::create_dir_all(&dst).unwrap();
        for entry in std::fs::read_dir(&src).unwrap() {
            let entry = entry.unwrap();
            std::fs::copy(entry.path(), dst.join(entry.file_name())).unwrap();
        }
    }
}

/// Every recorded Anthropic case replays to the outcome it reached when recorded.
#[test]
fn replay_anthropic_recordings() {
    let mut replayed = BTreeSet::new();
    for case in anthropic_cases() {
        let dir = fixture_dir("anthropic-messages", case.name);
        if !dir.join("expected.json").exists() {
            continue;
        }
        let expected: Expected =
            serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap())
                .unwrap();
        let player = Player::load(&dir, AnthropicProvider::volatile()).unwrap();
        let (_, reached) = run_case(&case, Arc::new(player), None, &replay_env);
        assert_eq!(reached.terminal, expected.terminal, "{}", case.name);
        assert_eq!(reached.verdict, expected.verdict, "{}", case.name);
        assert_eq!(reached.error, expected.error, "{}", case.name);
        assert_eq!(reached.repairs, expected.repairs, "{}", case.name);
        assert_eq!(reached.turns, expected.turns, "{}", case.name);
        assert_eq!(reached.tool_calls, expected.tool_calls, "{}", case.name);
        replayed.insert(case.name);
    }
    eprintln!(
        "replayed {} Anthropic recordings: {replayed:?}",
        replayed.len()
    );
}

/// The profile-conformance fixture replays like the case it was taken from.
#[test]
fn replay_anthropic_profile_fixture() {
    let dir = root().join("fixtures/profiles/anthropic");
    if !dir.join("expected.json").exists() {
        return;
    }
    let case = anthropic_cases()
        .into_iter()
        .find(|c| c.name == "first-look")
        .unwrap();
    let expected: Expected =
        serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap()).unwrap();
    let player = Player::load(&dir, Volatile::common()).unwrap();
    let (_, reached) = run_case(&case, Arc::new(player), None, &replay_env);
    assert_eq!(reached.terminal, expected.terminal);
    assert_eq!(reached.verdict, expected.verdict);
}
