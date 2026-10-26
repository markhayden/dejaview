//! Task 1.7a: tool turns through the run state machine with a scripted provider and a scripted
//! tool: first look then submit, two tool turns, a batch over the call budget, invalid arguments,
//! a tool crash, and the provider-history invariant.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use dejaview_core::config::Config;
use dejaview_core::job::Job;
use dejaview_core::provider::{ContentBlock, MessageRole, TurnResponse};
use dejaview_core::run;
use dejaview_core::tool::{Envelope, Tool, ToolRegistry};
use dejaview_testkit::{ScriptedProvider, ScriptedTool};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tempdir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("dejaview-tool-turns-{tag}-{}", std::process::id()));
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
    let turns: Vec<TurnResponse> = serde_json::from_str(&text).unwrap();
    ScriptedProvider::new(turns)
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

fn report(dir: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("report.json")).unwrap()).unwrap()
}

fn journal_events(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("run-1/journal.jsonl"))
        .unwrap()
        .lines()
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).unwrap()["event"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn two_tool_turns_then_submit() {
    let out = tempdir("two-turns");
    let p = provider("two-tool-turns.json");
    let t = tools("regions-3-then-1.json");
    let job = Job::load(&root().join("fixtures/jobs/pair.toml")).unwrap();
    let o = block_on(run::check(
        &config(&[]),
        &job,
        &root(),
        &run::Services::new(&p, &t),
        &out,
        "test",
    ))
    .unwrap();
    assert_eq!(o.exit_code, 0, "{:?}", o.error);
    let r = report(&o.dir);
    let run = &r["runs"][0];
    assert_eq!(run["turns"].as_array().unwrap().len(), 3);
    assert_eq!(run["tool_calls"].as_array().unwrap().len(), 2);
    assert_eq!(
        run["tool_calls"][0]["added_items"],
        serde_json::json!(["chk:ev:1:0", "chk:ev:1:1"])
    );
    assert_eq!(run["tool_calls"][0]["dropped"], serde_json::json!(["r2"]));
    assert_eq!(
        run["tool_calls"][1]["added_items"],
        serde_json::json!(["chk:ev:2:0"])
    );
    let ids: Vec<&str> = run["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "chk:ob:SF1@candidate",
            "chk:ob:SF2@candidate",
            "chk:ev:1:0",
            "chk:ev:1:1",
            "chk:ev:2:0"
        ]
    );
    let events = journal_events(&o.dir);
    assert_eq!(events.iter().filter(|e| *e == "tool_called").count(), 2);
    assert_eq!(
        events.iter().filter(|e| *e == "evidence_dropped").count(),
        1
    );
    assert!(events.contains(&"prepared".to_string()) && events.contains(&"reserved".to_string()));
    assert_eq!(r["tools"], serde_json::json!(["fake_regions@0.0.1"]));
    assert_eq!(r["prompt_version"], "v1");

    // Provider history is append-only: every request extends the previous one; the system prompt
    // and the tool definitions never change.
    let recorded = p.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 3);
    for pair in recorded.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        assert_eq!(a.system, b.system);
        assert_eq!(a.tools, b.tools, "tool definitions are frozen");
        assert!(b.history.len() > a.history.len());
        assert_eq!(
            &b.history[..a.history.len()],
            &a.history[..],
            "earlier messages are unchanged"
        );
    }
    assert_eq!(
        recorded[0]
            .tools
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["fake_regions", "submit_verdict"]
    );
    assert!(
        recorded[0].system.contains("Tools."),
        "the Tools section is rendered"
    );
    // The second request carries the first call's result and the updated checklist.
    let last = recorded[1].history.last().unwrap();
    assert_eq!(last.role, MessageRole::User);
    assert!(matches!(
        &last.content[0],
        ContentBlock::ToolResult {
            is_error: false,
            ..
        }
    ));
    assert!(matches!(&last.content[1], ContentBlock::Text { text } if text.contains("chk:ev:1:0")));
}

#[test]
fn first_look_then_submit_with_no_tools_enabled() {
    let out = tempdir("no-tools");
    let p = provider("held.json");
    let t = ToolRegistry::new(vec![], &[], &[]).unwrap();
    let job = Job::load(&root().join("fixtures/jobs/pair.toml")).unwrap();
    let o = block_on(run::check(
        &config(&["tools.enabled=[]"]),
        &job,
        &root(),
        &run::Services::new(&p, &t),
        &out,
        "test",
    ))
    .unwrap();
    assert_eq!(o.exit_code, 0, "{:?}", o.error);
    let recorded = p.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].tools.len(), 1, "only submit_verdict");
    assert!(
        !recorded[0].system.contains("Tools."),
        "no Tools section without tools"
    );
}

#[test]
fn a_batch_over_the_call_budget_is_answered_with_errors_then_the_final_turn() {
    let out = tempdir("over-budget");
    let p = provider("batch-over-budget.json");
    let t = tools("regions-3-then-1.json");
    let job = Job::load(&root().join("fixtures/jobs/pair.toml")).unwrap();
    let o = block_on(run::check(
        &config(&["budget.tool_calls=1"]),
        &job,
        &root(),
        &run::Services::new(&p, &t),
        &out,
        "test",
    ))
    .unwrap();
    assert_eq!(o.exit_code, 0, "{:?}", o.error);
    let recorded = p.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 2);
    let after_batch = &recorded[1].history;
    // assistant (2 calls), user (2 error results), user (final instruction)
    let results = &after_batch[after_batch.len() - 2];
    assert_eq!(
        results
            .content
            .iter()
            .filter(|c| matches!(c, ContentBlock::ToolResult { is_error: true, .. }))
            .count(),
        2
    );
    let instruction = &after_batch[after_batch.len() - 1];
    assert!(
        matches!(&instruction.content[0], ContentBlock::Text { text } if text.contains("Only submit_verdict"))
    );
    let r = report(&o.dir);
    assert_eq!(
        r["runs"][0]["tool_calls"].as_array().unwrap().len(),
        0,
        "nothing was executed"
    );
}

#[test]
fn invalid_arguments_are_error_results_not_failures() {
    let out = tempdir("invalid");
    let p = provider("invalid-args.json");
    let t = tools("regions-3-then-1.json");
    let job = Job::load(&root().join("fixtures/jobs/pair.toml")).unwrap();
    let o = block_on(run::check(
        &config(&[]),
        &job,
        &root(),
        &run::Services::new(&p, &t),
        &out,
        "test",
    ))
    .unwrap();
    assert_eq!(o.exit_code, 0, "{:?}", o.error);
    let r = report(&o.dir);
    let calls = r["runs"][0]["tool_calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(
        calls[0]["invalid_args"]
            .as_str()
            .unwrap()
            .contains("_dejaview")
    );
    assert!(
        calls[1]["invalid_args"]
            .as_str()
            .unwrap()
            .contains("unknown image id")
    );
    let recorded = p.recorded.lock().unwrap();
    let after_first = &recorded[1].history;
    assert!(
        matches!(&after_first.last().unwrap().content[0], ContentBlock::ToolResult { is_error: true, content, .. } if content.contains("_dejaview"))
    );
}

#[test]
fn a_tool_crash_is_an_operational_error() {
    let out = tempdir("crash");
    let p = provider("tool-crash.json");
    let t = tools("empty.json");
    let job = Job::load(&root().join("fixtures/jobs/pair.toml")).unwrap();
    let o = block_on(run::check(
        &config(&[]),
        &job,
        &root(),
        &run::Services::new(&p, &t),
        &out,
        "test",
    ))
    .unwrap();
    assert_eq!(o.exit_code, 3);
    let r = report(&o.dir);
    assert_eq!(r["runs"][0]["error"]["class"], "tool");
    assert!(r["verdict"].is_null());
}
