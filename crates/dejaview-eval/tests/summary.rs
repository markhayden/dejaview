//! Task 3.5a: the Markdown report is a pure function of the summary and the metrics, checked
//! against a stored rendering of the hand-computed fixture (`UPDATE_GOLDEN=1` rewrites it).
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use dejaview_eval::gate_rules::{Computed, Policy};
use dejaview_eval::metrics::{Expected, ItemRow, Region, compute};
use dejaview_eval::runner::Summary;
use dejaview_eval::summary::render;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(serde::Deserialize)]
struct Fixture {
    cell: String,
    policy: Policy,
    rows: Vec<FixtureRow>,
}

#[derive(serde::Deserialize)]
struct FixtureRow {
    item: String,
    repetition: u32,
    expected: Expected,
    computed: Computed,
    cost_usd: f64,
    wall_ms: u64,
    #[serde(default)]
    labels: Vec<Region>,
    #[serde(default)]
    predictions: Vec<Region>,
}

#[test]
fn the_report_renders_the_fixture_exactly_as_stored() {
    let text = std::fs::read_to_string(root().join("fixtures/stats/metrics-1.toml")).unwrap();
    let f: Fixture = toml::from_str(&text).unwrap();
    let rows: Vec<ItemRow> = f
        .rows
        .iter()
        .map(|r| ItemRow {
            set: "fixture".into(),
            item: r.item.clone(),
            cell: f.cell.clone(),
            repetition: r.repetition,
            expected: r.expected,
            computed: r.computed,
            cost_usd: r.cost_usd,
            wall_ms: r.wall_ms,
            labels: r.labels.clone(),
            predictions: r.predictions.clone(),
        })
        .collect();
    let m = compute(&rows, &f.policy);
    let summary = Summary {
        summary_version: 1,
        matrix: "metrics-1".into(),
        matrix_identity: "0".repeat(64),
        descriptor_sha256: "1".repeat(64),
        decisions: Vec::new(),
        completed: 10,
        skipped: 0,
        partial: Vec::new(),
        cancelled: false,
        budget_exhausted: false,
        spent_usd: 1.5,
        cap_usd: 5.0,
    };
    let rendered = render(&summary, &m);
    let golden = root().join("fixtures/stats/metrics-1.md");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &rendered).unwrap();
    }
    let stored = std::fs::read_to_string(&golden).unwrap();
    assert_eq!(
        rendered, stored,
        "render differs from fixtures/stats/metrics-1.md"
    );
    assert!(rendered.contains("| Detected failures | 1 / 3 | 0.333 |"));
    assert!(rendered.contains("2 hits of 2 labelled regions, 1 spurious of 3 predictions"));
    assert!(rendered.contains("agree with repetition 1 on 0.500 of items"));
    // The same inputs render the same bytes.
    assert_eq!(render(&summary, &m), rendered);
}
