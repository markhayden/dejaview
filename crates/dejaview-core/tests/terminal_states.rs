//! Task 1.7b: the final turn's refuse mechanism and every error class, each reached by a scripted
//! path, with the journal order asserted. Since task 1.8 the model-output failures fail twice: the
//! first failure is repaired.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use dejaview_core::config::Config;
use dejaview_core::job::Job;
use dejaview_core::provider::{ContentBlock, TurnResponse};
use dejaview_core::run;
use dejaview_core::tool::{Envelope, Tool, ToolRegistry};
use dejaview_testkit::{ScriptedProvider, ScriptedTool};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dejaview-terminal-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn config(overrides: &[&str]) -> Config {
    let text = std::fs::read_to_string(root().join("fixtures/config/fake-tool.toml")).unwrap();
    let ov: Vec<String> = overrides.iter().map(|s| s.to_string()).collect();
    Config::parse(&text, &ov, &|_| None).unwrap()
}

fn provider(script: &str) -> ScriptedProvider {
    let text = std::fs::read_to_string(root().join("fixtures/scripts").join(script)).unwrap();
    ScriptedProvider::new(serde_json::from_str::<Vec<TurnResponse>>(&text).unwrap())
}

fn tools(tool_script: &str) -> ToolRegistry {
    let text =
        std::fs::read_to_string(root().join("fixtures/tool-scripts").join(tool_script)).unwrap();
    let envelopes: Vec<Envelope> = serde_json::from_str(&text).unwrap();
    let t: Vec<Box<dyn Tool>> = vec![Box::new(ScriptedTool::fake_regions(envelopes))];
    ToolRegistry::new(t, &["fake_regions".into()], &[]).unwrap()
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

fn events(dir: &std::path::Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(dir.join("run-1/journal.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn names(ev: &[serde_json::Value]) -> Vec<&str> {
    ev.iter().map(|e| e["event"].as_str().unwrap()).collect()
}

fn run_case(
    tag: &str,
    script: &str,
    tool_script: &str,
    overrides: &[&str],
    job: &str,
) -> (run::Outcome, ScriptedProvider) {
    let out = tempdir(tag);
    let p = provider(script);
    let t = tools(tool_script);
    let job = Job::load(&root().join("fixtures/jobs").join(job)).unwrap();
    let o = block_on(run::check(
        &config(overrides),
        &job,
        &root(),
        &p,
        &t,
        &out,
        "test",
        None,
    ))
    .unwrap();
    (o, p)
}

fn error_class(o: &run::Outcome) -> String {
    let r: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(o.dir.join("report.json")).unwrap()).unwrap();
    r["runs"][0]["error"]["class"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

#[test]
fn a_stray_call_on_the_final_turn_gets_an_error_result_and_the_run_ends_no_submission() {
    let (o, p) = run_case(
        "stray",
        "stray-final-call.json",
        "regions-3-then-1.json",
        &["budget.tool_turns=1"],
        "pair.toml",
    );
    assert_eq!(o.exit_code, 3);
    assert_eq!(error_class(&o), "no_submission");
    let ev = events(&o.dir);
    assert_eq!(
        names(&ev),
        [
            "resolved",
            "prepared",
            "reserved",
            "turn_sent",
            "turn_received",
            "tool_called",
            "evidence_dropped",
            "tool_returned",
            "turn_sent",
            "turn_received",
            "tool_called",
            "repair",
            "turn_sent",
            "turn_received",
            "tool_called",
            "error"
        ]
    );
    let stray = &ev[10];
    assert_eq!(stray["data"]["executed"], false);
    assert_eq!(stray["data"]["reason"], "final_turn");
    let recorded = p.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 3);
    let final_instruction = recorded[1].history.last().unwrap();
    assert!(matches!(
        &final_instruction.content[0],
        ContentBlock::Text { text } if text.contains("Only submit_verdict will be executed now")
    ));
    assert_eq!(
        recorded[0].tools, recorded[2].tools,
        "the final turn and its repair keep the frozen tool definitions"
    );
}

#[test]
fn a_tool_crash_is_terminal_even_when_the_script_would_submit_afterwards() {
    let (o, p) = run_case(
        "crash",
        "crash-then-submit.json",
        "empty.json",
        &[],
        "pair.toml",
    );
    assert_eq!(o.exit_code, 3);
    assert_eq!(error_class(&o), "tool");
    assert_eq!(p.remaining(), 1, "the valid submission was never requested");
    assert_eq!(names(&events(&o.dir)).last().copied(), Some("error"));
}

#[test]
fn an_exhausted_provider_is_a_provider_error() {
    // Two truncated turns, two repairs allowed: the third request finds the script empty.
    let (o, _) = run_case(
        "provider",
        "truncated.json",
        "regions-3-then-1.json",
        &["budget.repairs=2"],
        "pair.toml",
    );
    assert_eq!(o.exit_code, 3);
    assert_eq!(error_class(&o), "provider");
}

#[test]
fn a_bad_image_is_a_resolve_error() {
    let (o, _) = run_case("resolve", "held.json", "empty.json", &[], "oversized.toml");
    assert_eq!(o.exit_code, 3);
    assert_eq!(error_class(&o), "resolve");
    assert_eq!(
        names(&events(&o.dir)),
        ["error"],
        "nothing else happened before the resolve error"
    );
}

#[test]
fn a_second_model_output_failure_after_the_repair_is_terminal() {
    for (tag, script) in [
        ("text", "text-no-call.json"),
        ("trunc", "truncated.json"),
        ("mixed", "mixed-batch.json"),
    ] {
        let (o, _) = run_case(tag, script, "regions-3-then-1.json", &[], "pair.toml");
        assert_eq!(o.exit_code, 3, "{script}");
        assert_eq!(error_class(&o), "model_output", "{script}");
    }
}

#[test]
fn the_evidence_budget_is_a_budget_error_before_presentation() {
    let (o, p) = run_case(
        "budget",
        "one-call.json",
        "regions-25.json",
        &[],
        "pair.toml",
    );
    assert_eq!(o.exit_code, 3);
    assert_eq!(error_class(&o), "budget");
    assert_eq!(
        p.recorded.lock().unwrap().len(),
        1,
        "the result was never presented"
    );
}

#[test]
fn an_invalid_submission_is_submission_invalid() {
    let (o, _) = run_case(
        "invalid",
        "missing-item.json",
        "empty.json",
        &["tools.enabled=[]"],
        "pair.toml",
    );
    assert_eq!(o.exit_code, 3);
    assert_eq!(error_class(&o), "submission_invalid");
}
