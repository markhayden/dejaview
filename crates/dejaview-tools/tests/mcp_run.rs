//! Task 4.4a through the run: a batch of calls to pack tools that declare `concurrent = true`
//! starts together and settles in call order; without the declaration the same batch runs one
//! call after another.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use dejaview_core::config::Config;
use dejaview_core::job::Job;
use dejaview_core::run;
use dejaview_core::tool::ToolRegistry;
use dejaview_testkit::{ScriptedProvider, mcp_server};
use dejaview_tools::mcp::{Pack, Spawn, Timeouts};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dejaview-mcp-run-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Two scripted region calls in one batch, then a submission citing both.
fn script() -> serde_json::Value {
    serde_json::json!([
        {
            "tool_calls": [
                { "id": "c1", "name": "fake_regions", "arguments": { "image": "candidate" } },
                { "id": "c2", "name": "fake_regions", "arguments": { "image": "candidate" } }
            ],
            "stop_reason": "tool_use"
        },
        {
            "tool_calls": [{ "id": "c_submit", "name": "submit_verdict", "arguments": {
                "schema_version": 1,
                "items": [
                    { "id": "chk:ob:SF1@candidate", "disposition": "held", "confidence": "confident", "reason": "scripted", "allowed_by": null, "location": null, "evidence": ["call:1", "call:2"] },
                    { "id": "chk:ob:SF2@candidate", "disposition": "held", "confidence": "confident", "reason": "scripted", "allowed_by": null, "location": null, "evidence": [] },
                    { "id": "chk:ev:1:0", "disposition": "held", "confidence": "confident", "reason": "scripted", "allowed_by": null, "location": null, "evidence": [] },
                    { "id": "chk:ev:2:0", "disposition": "held", "confidence": "confident", "reason": "scripted", "allowed_by": null, "location": null, "evidence": [] }
                ],
                "findings": [],
                "opinion": { "verdict": "pass", "reason": "r" }
            } }],
            "stop_reason": "tool_use"
        }
    ])
}

fn batch_through_the_run(flags: &[&str], tag: &str) -> (Duration, serde_json::Value) {
    let out = tempdir(tag);
    let mut args: Vec<String> = vec![
        "--envelopes".into(),
        // One region per envelope, so the evidence items are the same whichever call the server
        // happens to serve first.
        root()
            .join("fixtures/tool-scripts/regions-1-then-1.json")
            .display()
            .to_string(),
        "--slow-call-ms".into(),
        "400".into(),
    ];
    args.extend(flags.iter().map(|s| s.to_string()));
    let pack = Pack::connect(
        "fake",
        Spawn {
            command: mcp_server::path(),
            args,
            declared_env: vec![],
            cwd: None,
            extra_env: vec![],
        },
        Timeouts {
            startup: Duration::from_secs(10),
            call: Duration::from_secs(5),
            shutdown_grace: Duration::from_millis(300),
        },
        std::env::vars(),
    )
    .unwrap();
    let registry = ToolRegistry::new(pack.tools(), &["fake_regions".into()], &[]).unwrap();
    let provider = ScriptedProvider::new(serde_json::from_value(script()).unwrap());
    let config = Config::parse(
        &std::fs::read_to_string(root().join("fixtures/config/fake-tool.toml")).unwrap(),
        &["budget.spend_cap_usd=100".into()],
        &|_| None,
    )
    .unwrap();
    let job = Job::load(&root().join("fixtures/jobs/pair.toml")).unwrap();
    let started = Instant::now();
    let o = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run::check(
            &config,
            &job,
            &root(),
            &run::Services::new(&provider, &registry),
            &out,
            "test",
        ))
        .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(o.exit_code, 0, "{:?}", o.error);
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(o.dir.join("report.json")).unwrap()).unwrap();
    (elapsed, report)
}

#[test]
fn a_batch_of_concurrent_pack_tools_runs_together_and_settles_in_call_order() {
    let (elapsed, report) = batch_through_the_run(&["--concurrent"], "together");
    assert!(
        elapsed < Duration::from_millis(1500),
        "two 400 ms calls together, not in turn (plus the run's own work): {elapsed:?}"
    );
    let calls = report["runs"][0]["tool_calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        (calls[0]["id"].as_str(), calls[1]["id"].as_str()),
        (Some("call:1"), Some("call:2"))
    );
    // One region from each scripted envelope, in call order.
    assert_eq!(calls[0]["added_items"].as_array().unwrap().len(), 1);
    assert_eq!(calls[1]["added_items"].as_array().unwrap().len(), 1);
    let items: Vec<&str> = report["runs"][0]["checklist"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["id"].as_str())
        .filter(|id| id.starts_with("chk:ev:"))
        .collect();
    assert_eq!(items, ["chk:ev:1:0", "chk:ev:2:0"]);
}

#[test]
fn a_batch_of_serial_pack_tools_runs_one_call_after_another() {
    let (elapsed, report) = batch_through_the_run(&[], "in-turn");
    assert!(elapsed >= Duration::from_millis(800), "{elapsed:?}");
    assert_eq!(report["runs"][0]["tool_calls"].as_array().unwrap().len(), 2);
}
