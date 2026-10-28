//! Task 3.5a: metrics per cell from the decision rows (SPEC "Metrics"), against a hand-computed
//! fixture; localisation eligibility rules; the headline from repetition 1 only.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use dejaview_eval::gate_rules::{Computed, Policy};
use dejaview_eval::metrics::{self, Expected, ItemRow, Region, RegionKind, compute};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(serde::Deserialize)]
struct Fixture {
    cell: String,
    policy: Policy,
    rows: Vec<FixtureRow>,
    golden: toml::Table,
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

fn rows(f: &Fixture) -> Vec<ItemRow> {
    f.rows
        .iter()
        .map(|r| ItemRow {
            set: "s".into(),
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
        .collect()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn the_fixture_metrics_match_the_hand_computed_numbers() {
    let text = std::fs::read_to_string(root().join("fixtures/stats/metrics-1.toml")).unwrap();
    let f: Fixture = toml::from_str(&text).unwrap();
    let m = compute(&rows(&f), &f.policy);
    assert_eq!(m.cells.len(), 1);
    let c = &m.cells[0];
    let g = &f.golden;
    let int = |k: &str| g[k].as_integer().unwrap() as usize;
    let num = |k: &str| g[k].as_float().unwrap();
    assert_eq!(c.cell, f.cell);
    assert_eq!(c.counts.items, int("items"));
    assert_eq!(c.counts.expected_fail, int("expected_fail"));
    assert_eq!(c.counts.expected_pass, int("expected_pass"));
    assert_eq!(c.counts.detected, int("detected"));
    assert_eq!(c.counts.missed, int("missed"));
    assert_eq!(c.counts.false_rejects, int("false_rejects"));
    assert_eq!(c.counts.unsure, int("unsure"));
    assert_eq!(c.counts.errors, int("errors"));
    assert!(close(c.rates.missed.value, num("missed_rate")));
    assert!(close(c.rates.detected.value, num("detected_rate")));
    assert!(close(c.rates.false_rejects.value, num("false_reject_rate")));
    assert!(close(c.rates.unsure.value, num("unsure_rate")));
    assert!(close(c.rates.errors.value, num("error_rate")));
    // Wilson intervals to 1e-9 against the independently computed values.
    assert!(
        close(c.rates.missed.low, num("missed_low"))
            && close(c.rates.missed.high, num("missed_high"))
    );
    assert!(
        close(c.rates.unsure.low, num("unsure_low"))
            && close(c.rates.unsure.high, num("unsure_high"))
    );
    assert!(close(c.rates.errors.low, num("unsure_low")), "1/6 as well");
    assert!(close(c.cost.total_usd, num("cost_total_usd")));
    assert!(close(c.cost.median_usd, num("cost_median_usd")));
    assert_eq!(c.wall.total_ms, int("wall_total_ms") as u64);
    assert_eq!(c.wall.median_ms, int("wall_median_ms") as u64);
    assert_eq!(c.localisation.labelled_regions, int("labelled_regions"));
    assert_eq!(c.localisation.hits, int("hits"));
    assert_eq!(c.localisation.predictions, int("predictions"));
    assert_eq!(c.localisation.spurious, int("spurious"));
    assert_eq!(c.repetitions.count, int("repetitions") as u32);
    assert!(close(c.repetitions.agreement.unwrap(), num("agreement")));
    // The confusion table: expected pass/fail rows, computed pass/fail/unsure/error columns.
    assert_eq!(c.confusion.expected_fail, [1, 1, 1, 0]);
    assert_eq!(c.confusion.expected_pass, [1, 1, 0, 1]);
}

fn region(image: &str, kind: RegionKind, bbox: Option<[f64; 4]>, ids: &[&str]) -> Region {
    Region {
        image: image.into(),
        kind,
        bbox,
        polygon: None,
        obligation_ids: ids.iter().map(|s| s.to_string()).collect(),
    }
}

fn detected_row(labels: Vec<Region>, predictions: Vec<Region>) -> ItemRow {
    ItemRow {
        set: "s".into(),
        item: "i".into(),
        cell: "c".into(),
        repetition: 1,
        expected: Expected::Fail,
        computed: Computed::Fail,
        cost_usd: 0.0,
        wall_ms: 1,
        labels,
        predictions,
    }
}

#[test]
fn localisation_eligibility_rules_each_have_a_case() {
    let policy = Policy {
        on_unsure: dejaview_core::config::PolicyOutcome::Review,
        on_error: dejaview_core::config::PolicyOutcome::Review,
    };
    let l = |m: &metrics::MatrixMetrics| m.cells[0].localisation.clone();
    // Below the IoU threshold: not a hit, the prediction is spurious.
    let far = detected_row(
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.1, 0.1, 0.3, 0.3]),
            &["SF1"],
        )],
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.25, 0.25, 0.45, 0.45]),
            &["SF1"],
        )],
    );
    let r = l(&compute(&[far], &policy));
    assert_eq!((r.hits, r.spurious), (0, 1));
    // Obligations must intersect: the same box under another obligation is ineligible.
    let other = detected_row(
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.1, 0.1, 0.3, 0.3]),
            &["SF1"],
        )],
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.1, 0.1, 0.3, 0.3]),
            &["SF2"],
        )],
    );
    let r = l(&compute(&[other], &policy));
    assert_eq!((r.hits, r.spurious), (0, 1));
    // The root image must match: a perfect box on the source does not hit a candidate label.
    let elsewhere = detected_row(
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.1, 0.1, 0.3, 0.3]),
            &["SF1"],
        )],
        vec![region(
            "source",
            RegionKind::Bbox,
            Some([0.1, 0.1, 0.3, 0.3]),
            &["SF1"],
        )],
    );
    let r = l(&compute(&[elsewhere], &policy));
    assert_eq!((r.hits, r.spurious), (0, 1));
    // whole_image and absent match only their own kind on the same image.
    let absent = detected_row(
        vec![region("candidate", RegionKind::Absent, None, &["SF1"])],
        vec![
            region("candidate", RegionKind::Absent, None, &["SF1"]),
            region("candidate", RegionKind::WholeImage, None, &["SF1"]),
        ],
    );
    let r = l(&compute(&[absent], &policy));
    assert_eq!((r.hits, r.spurious), (1, 1));
    // One label takes one prediction: the best IoU wins, the other is spurious.
    let two = detected_row(
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.1, 0.1, 0.3, 0.3]),
            &["SF1"],
        )],
        vec![
            region(
                "candidate",
                RegionKind::Bbox,
                Some([0.15, 0.15, 0.35, 0.35]),
                &["SF1"],
            ),
            region(
                "candidate",
                RegionKind::Bbox,
                Some([0.1, 0.1, 0.3, 0.3]),
                &["SF1"],
            ),
        ],
    );
    let r = l(&compute(&[two], &policy));
    assert_eq!(
        (r.hits, r.spurious, r.labelled_regions, r.predictions),
        (1, 1, 1, 2)
    );
    // A polygon prediction is matched by its bounding box.
    let mut poly = region("candidate", RegionKind::Polygon, None, &["SF1"]);
    poly.polygon = Some(vec![[0.1, 0.1], [0.3, 0.1], [0.3, 0.3], [0.1, 0.3]]);
    let polygon = detected_row(
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.1, 0.1, 0.3, 0.3]),
            &["SF1"],
        )],
        vec![poly],
    );
    let r = l(&compute(&[polygon], &policy));
    assert_eq!((r.hits, r.spurious), (1, 0));
    // Only detected failures count: a missed failure's labels are not scored.
    let mut missed = detected_row(
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.1, 0.1, 0.3, 0.3]),
            &["SF1"],
        )],
        vec![],
    );
    missed.computed = Computed::Pass;
    let r = l(&compute(&[missed], &policy));
    assert_eq!((r.hits, r.labelled_regions, r.predictions), (0, 0, 0));
}

#[test]
fn later_repetitions_never_move_the_headline() {
    let policy = Policy {
        on_unsure: dejaview_core::config::PolicyOutcome::Review,
        on_error: dejaview_core::config::PolicyOutcome::Review,
    };
    let row = |cell: &str, item: &str, rep: u32, computed: Computed| ItemRow {
        set: "s".into(),
        item: item.into(),
        cell: cell.into(),
        repetition: rep,
        expected: Expected::Fail,
        computed,
        cost_usd: 0.0,
        wall_ms: 1,
        labels: vec![],
        predictions: vec![],
    };
    // Repetition 1: cell a detects both, cell b detects none. Repetition 2 says the opposite.
    let rows = vec![
        row("a", "i1", 1, Computed::Fail),
        row("a", "i2", 1, Computed::Fail),
        row("b", "i1", 1, Computed::Pass),
        row("b", "i2", 1, Computed::Pass),
        row("a", "i1", 2, Computed::Pass),
        row("a", "i2", 2, Computed::Pass),
        row("b", "i1", 2, Computed::Fail),
        row("b", "i2", 2, Computed::Fail),
    ];
    let m = compute(&rows, &policy);
    let a = m.cells.iter().find(|c| c.cell == "a").unwrap();
    let b = m.cells.iter().find(|c| c.cell == "b").unwrap();
    assert_eq!(
        (a.counts.detected, b.counts.detected),
        (2, 0),
        "headline from repetition 1"
    );
    assert!(
        close(a.repetitions.agreement.unwrap(), 0.0)
            && close(b.repetitions.agreement.unwrap(), 0.0)
    );
}

#[test]
fn the_frozen_policy_decides_what_counts_as_missed() {
    let text = std::fs::read_to_string(root().join("fixtures/stats/metrics-1.toml")).unwrap();
    let f: Fixture = toml::from_str(&text).unwrap();
    let rows = rows(&f);
    // on_unsure = pass: i3 (expected fail, unsure) is let through, so missed is 2 of 3.
    let lenient = Policy {
        on_unsure: dejaview_core::config::PolicyOutcome::Pass,
        on_error: dejaview_core::config::PolicyOutcome::Review,
    };
    let c = &compute(&rows, &lenient).cells[0];
    assert_eq!(
        (c.counts.missed, c.counts.detected, c.counts.unsure),
        (2, 1, 1)
    );
    assert!(close(c.rates.missed.value, 2.0 / 3.0));
    assert_eq!(
        c.confusion.expected_fail,
        [1, 1, 1, 0],
        "the confusion does not move with the policy"
    );
    // on_error = fail: i6 (expected pass, error) is not a computed fail, so false rejects stay 1.
    let strict = Policy {
        on_unsure: dejaview_core::config::PolicyOutcome::Review,
        on_error: dejaview_core::config::PolicyOutcome::Fail,
    };
    let c = &compute(&rows, &strict).cells[0];
    assert_eq!(
        (c.counts.missed, c.counts.false_rejects, c.counts.errors),
        (1, 1, 1)
    );
    assert_eq!(c.confusion.expected_pass, [1, 1, 0, 1]);
}

#[test]
fn an_iou_at_the_threshold_is_not_a_hit_and_an_empty_cell_does_not_panic() {
    let policy = Policy {
        on_unsure: dejaview_core::config::PolicyOutcome::Review,
        on_error: dejaview_core::config::PolicyOutcome::Review,
    };
    // Label [0,0,1,1] and prediction [0,0,0.3,1]: IoU exactly 0.3.
    let at = detected_row(
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.0, 0.0, 1.0, 1.0]),
            &["SF1"],
        )],
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.0, 0.0, 0.3, 1.0]),
            &["SF1"],
        )],
    );
    let r = compute(&[at], &policy).cells[0].localisation.clone();
    assert_eq!((r.hits, r.spurious), (0, 1));
    let above = detected_row(
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.0, 0.0, 1.0, 1.0]),
            &["SF1"],
        )],
        vec![region(
            "candidate",
            RegionKind::Bbox,
            Some([0.0, 0.0, 0.31, 1.0]),
            &["SF1"],
        )],
    );
    assert_eq!(compute(&[above], &policy).cells[0].localisation.hits, 1);
    // A cell with expected passes only: the fail-based rates are 0 of 0 with the whole interval.
    let mut pass_only = detected_row(vec![], vec![]);
    pass_only.expected = Expected::Pass;
    pass_only.computed = Computed::Pass;
    let c = &compute(&[pass_only], &policy).cells[0];
    assert_eq!((c.counts.expected_fail, c.rates.missed.total), (0, 0));
    assert_eq!(
        (
            c.rates.missed.value,
            c.rates.missed.low,
            c.rates.missed.high
        ),
        (0.0, 0.0, 1.0)
    );
    assert_eq!(c.localisation.labelled_regions, 0);
    assert!(compute(&[], &policy).cells.is_empty());
}
