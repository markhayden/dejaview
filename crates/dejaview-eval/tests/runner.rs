//! Task 3.4b: the matrix runner with the fake provider: every cell runs, the cache is never
//! read, the cap stops it with `error: budget` per unstarted decision, a cancelled run resumes
//! without redoing complete decisions and marks partial ones, repetition 1 is the decision
//! repetition, and prompts across arms differ only in the Tools section.
#![allow(clippy::unwrap_used)]

mod common;

use std::path::Path;

use common::{Fake, plan, root, tempdir};
use dejaview_core::run::Cancel;
use dejaview_core::tool::{Tool, ToolRegistry};
use dejaview_eval::runner::{Summary, run};
use dejaview_testkit::ScriptedTool;

fn fake(script: &str) -> Fake {
    Fake::new(script)
}

fn summary(dir: &Path) -> Summary {
    serde_json::from_str(&std::fs::read_to_string(dir.join("summary.json")).unwrap()).unwrap()
}

#[test]
fn every_cell_runs_with_the_cache_bypassed_and_repetition_one_flagged() {
    let dir = tempdir("cells");
    let s = run(&plan(&dir, "fake-a.toml"), &fake("held.json"), None).unwrap();
    assert_eq!(s.decisions.len(), 4, "2 items × 1 cell × 2 repetitions");
    assert_eq!(s.completed, 4);
    assert_eq!(s.skipped, 0);
    assert!(s.partial.is_empty());
    for d in &s.decisions {
        assert_eq!(d.verdict.as_deref(), Some("pass"), "{d:?}");
        assert_eq!(d.exit_code, 0);
        assert_eq!(d.cell, "fake:fake-model/A");
        assert_eq!(d.decision_repetition, d.repetition == 1);
        let report: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join(&d.dir).join("report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            report["cache"]["status"], "bypass",
            "an evaluation never reads the cache"
        );
        assert!(d.wall_ms > 0 || d.cost_usd >= 0.0);
    }
    assert_eq!(summary(&dir), s);
    assert!(dir.join("descriptor.json").exists() && dir.join("ledger.jsonl").exists());
    assert!(!dir.join("LOCK").exists(), "the lock is released");
    // The run ends with the metrics and the Markdown report, which `eval report` reproduces
    // byte for byte from the saved results and the labels.
    let report_md = std::fs::read_to_string(dir.join("report.md")).unwrap();
    let metrics_json = std::fs::read_to_string(dir.join("metrics.json")).unwrap();
    let m: dejaview_eval::metrics::MatrixMetrics = serde_json::from_str(&metrics_json).unwrap();
    assert_eq!(m.cells.len(), 1);
    let c = &m.cells[0];
    assert_eq!(
        (
            c.counts.items,
            c.counts.expected_fail,
            c.counts.expected_pass
        ),
        (2, 1, 1)
    );
    assert_eq!(
        c.counts.missed, 1,
        "pair-02 is a rotation the scripted pass misses"
    );
    assert_eq!(c.repetitions.count, 2);
    assert_eq!(c.repetitions.agreement, Some(1.0));
    assert!(
        report_md.starts_with("# Evaluation report: fake-a\n"),
        "{report_md}"
    );
    std::fs::remove_file(dir.join("report.md")).unwrap();
    dejaview_eval::summary::regenerate(&dir, &root().join("fixtures/evalsets")).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("report.md")).unwrap(),
        report_md
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("metrics.json")).unwrap(),
        metrics_json
    );
    // The same matrix again: everything is complete, nothing is redone.
    let again = run(&plan(&dir, "fake-a.toml"), &fake("held.json"), None).unwrap();
    assert_eq!((again.completed, again.skipped), (0, 4));
}

#[test]
fn the_cap_stops_the_matrix_with_a_budget_error_per_unstarted_decision() {
    let dir = tempdir("cap");
    let mut p = plan(&dir, "fake-a.toml");
    p.matrix.matrix.spend_cap_usd = 0.2;
    p.matrix_text = p
        .matrix_text
        .replace("spend_cap_usd = 5.0", "spend_cap_usd = 0.2");
    let s = run(&p, &fake("held-costly.json"), None).unwrap();
    let budget: Vec<&_> = s
        .decisions
        .iter()
        .filter(|d| d.error_class.as_deref() == Some("budget"))
        .collect();
    assert!(!budget.is_empty() && budget.len() < 4, "{s:?}");
    assert!(
        s.decisions
            .iter()
            .any(|d| d.verdict.as_deref() == Some("pass")),
        "the first decision fits"
    );
    assert!(s.spent_usd <= 0.2 + 1e-9);
}

#[test]
fn a_cancelled_run_resumes_without_redoing_complete_decisions_and_marks_partial_ones() {
    let dir = tempdir("resume");
    let cancel = Cancel::new();
    let mut f = fake("held.json");
    f.cancel_after = Some((2, cancel.clone()));
    let s = run(&plan(&dir, "fake-a.toml"), &f, Some(&cancel)).unwrap();
    assert!(s.cancelled, "{s:?}");
    assert_eq!(s.completed, 2);
    // Decisions run item by item, then cell, then repetition: the third is pair-02's first.
    // Fake a crash mid-decision: its directory holds a started attempt and no result.
    let third = dir.join("results/source-fidelity/pairs-v1/pair-02/fake_fake-model/A/rep-1");
    assert!(!third.join("result.json").exists());
    std::fs::create_dir_all(third.join("decision-000-crashed/run-1")).unwrap();
    let s2 = run(&plan(&dir, "fake-a.toml"), &fake("held.json"), None).unwrap();
    assert!(!s2.cancelled);
    assert_eq!(s2.skipped, 2, "the two complete decisions are not redone");
    assert_eq!(s2.completed, 2);
    assert_eq!(
        s2.partial,
        ["source-fidelity/pairs-v1/pair-02/fake:fake-model/A/rep-1"]
    );
    assert_eq!(
        s2.decisions.len(),
        4,
        "the summary covers every decision, resumed ones included"
    );
}

#[test]
fn a_changed_input_refuses_resume() {
    let dir = tempdir("changed");
    run(&plan(&dir, "fake-a.toml"), &fake("held.json"), None).unwrap();
    let mut p = plan(&dir, "fake-a.toml");
    p.harness_version = "0.9.9".into();
    let err = run(&p, &fake("held.json"), None).unwrap_err();
    assert!(err.to_string().contains("different inputs"), "{err}");
}

#[test]
fn prompts_across_arms_differ_only_in_the_tools_section() {
    use dejaview_core::job::Job;
    use dejaview_core::obligations::{MergeInput, merge, resolve};
    use dejaview_core::render::{system_prompt, tools_section};
    let job = Job::load(
        &root().join("fixtures/evalsets/source-fidelity/pairs-v1/items/pair-01/job.toml"),
    )
    .unwrap();
    let merged = merge(&MergeInput {
        base: "source-fidelity",
        profile: None,
        extra: &[],
        extra_names: &[],
        job: Some(&job),
        enabled_tools: &[],
    })
    .unwrap();
    let resolved = resolve(&merged, &job).unwrap();
    let none = ToolRegistry::new(vec![], &[], &[]).unwrap();
    let text = std::fs::read_to_string(root().join("fixtures/tool-scripts/regions-3-then-1.json"))
        .unwrap();
    let with: Vec<Box<dyn Tool>> = vec![Box::new(ScriptedTool::fake_regions(
        serde_json::from_str(&text).unwrap(),
    ))];
    let with = ToolRegistry::new(with, &["fake_regions".into()], &[]).unwrap();
    let a = system_prompt(&resolved, &job, none.manifests());
    let d = system_prompt(&resolved, &job, with.manifests());
    assert_ne!(a, d);
    let a_stripped = a.replace(&tools_section(none.manifests()), "");
    let d_stripped = d.replace(&tools_section(with.manifests()), "");
    assert_eq!(
        a_stripped, d_stripped,
        "everything but the Tools section is identical"
    );
}

#[test]
fn a_model_account_in_the_matrix_binds_its_cells() {
    let dir = tempdir("account");
    let mut p = plan(&dir, "fake-a.toml");
    p.matrix_text = p.matrix_text.replace(
        "model = \"fake-model\"",
        "model = \"fake-model\"\naccount = \"fake:from-the-matrix\"",
    );
    p.matrix = dejaview_eval::matrix::Matrix::parse(&p.matrix_text).unwrap();
    let s = run(&p, &fake("held.json"), None).unwrap();
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(&s.decisions[0].dir).join("report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        report["config"]["privacy"]["account"],
        "fake:from-the-matrix"
    );
}
