//! Metrics per matrix cell over labelled items (SPEC "Metrics"): the confusion of expected
//! against computed, missed and detected failures under the frozen policy, false rejects, unsure
//! and error rates with Wilson intervals, localisation under the eligibility rules, cost and wall
//! totals and medians. Every headline number comes from repetition 1, the decision repetition;
//! later repetitions only contribute the per-item agreement.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::gate_rules::{Computed, Policy};
pub use crate::sets::{Expected, RegionKind};
use crate::wilson::{Z_95, wilson};

/// IoU above which an eligible pair of regions is a hit.
pub const HIT_IOU: f64 = 0.3;

/// A region on a root image in root coordinates: a labelled region, or a finding's location
/// mapped to the root image. A `bbox` or `polygon` region without geometry matches nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// The root image id (`candidate`, `source`, `ref:<label>`).
    pub image: String,
    pub kind: RegionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<Vec<[f64; 2]>>,
    #[serde(default)]
    pub obligation_ids: Vec<String>,
}

impl Region {
    /// The box used for IoU: the bbox, or the polygon's bounding box.
    fn boxed(&self) -> Option<[f64; 4]> {
        match self.kind {
            RegionKind::Bbox => self.bbox,
            RegionKind::Polygon => {
                let pts = self.polygon.as_ref()?;
                if pts.is_empty() {
                    return None;
                }
                let mut b = [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ];
                for [x, y] in pts {
                    b[0] = b[0].min(*x);
                    b[1] = b[1].min(*y);
                    b[2] = b[2].max(*x);
                    b[3] = b[3].max(*y);
                }
                Some(b)
            }
            RegionKind::WholeImage | RegionKind::Absent => None,
        }
    }
}

/// One decision of one labelled item: what the label says, what the harness computed, what it
/// cost, and the regions on both sides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRow {
    pub set: String,
    pub item: String,
    /// The item's family in its set, for the paired bootstrap; empty when unknown.
    #[serde(default)]
    pub family: String,
    pub cell: String,
    pub repetition: u32,
    pub expected: Expected,
    pub computed: Computed,
    pub cost_usd: f64,
    pub wall_ms: u64,
    /// The label's regions.
    #[serde(default)]
    pub labels: Vec<Region>,
    /// The violations the model submitted, in root coordinates.
    #[serde(default)]
    pub predictions: Vec<Region>,
}

/// A binomial proportion with its 95% Wilson interval. With no trials the value is 0 and the
/// interval is the whole unit interval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proportion {
    pub count: usize,
    pub total: usize,
    pub value: f64,
    pub low: f64,
    pub high: f64,
}

impl Proportion {
    pub fn of(count: usize, total: usize) -> Proportion {
        match wilson(count as u64, total as u64, Z_95) {
            Some((low, high)) => Proportion {
                count,
                total,
                value: count as f64 / total as f64,
                low,
                high,
            },
            None => Proportion {
                count,
                total,
                value: 0.0,
                low: 0.0,
                high: 1.0,
            },
        }
    }
}

/// Counts over the decision repetition.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub items: usize,
    pub expected_fail: usize,
    pub expected_pass: usize,
    /// Expected fail, computed fail.
    pub detected: usize,
    /// Expected fail, let through as a pass by the policy.
    pub missed: usize,
    /// Expected pass, computed fail.
    pub false_rejects: usize,
    pub unsure: usize,
    pub errors: usize,
}

/// Rows expected fail and pass; columns computed pass, fail, unsure, error.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Confusion {
    pub expected_fail: [usize; 4],
    pub expected_pass: [usize; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rates {
    /// Over expected fails.
    pub missed: Proportion,
    /// Over expected fails.
    pub detected: Proportion,
    /// Over expected passes.
    pub false_rejects: Proportion,
    /// Over all items.
    pub unsure: Proportion,
    /// Over all items.
    pub errors: Proportion,
}

/// Localisation over detected failures with labelled regions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Localisation {
    pub labelled_regions: usize,
    pub hits: usize,
    pub predictions: usize,
    pub spurious: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    /// Over every evaluated item, errors included.
    pub total_usd: f64,
    pub median_usd: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wall {
    pub total_ms: u64,
    /// The middle value, or the lower of the two middle values' mean rounded down.
    pub median_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repetitions {
    /// The highest repetition seen in the cell.
    pub count: u32,
    /// Of every (item, repetition above 1) pair, the share whose computed verdict equals the
    /// decision repetition's; `None` with one repetition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreement: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellMetrics {
    pub cell: String,
    pub counts: Counts,
    pub confusion: Confusion,
    pub rates: Rates,
    pub localisation: Localisation,
    pub cost: Cost,
    pub wall: Wall,
    pub repetitions: Repetitions,
}

/// One item whose computed verdict differs between the two arms of a paired comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flip {
    pub set: String,
    pub item: String,
    pub expected: Expected,
    pub from: Computed,
    pub to: Computed,
}

/// The seed and resample count of the paired bootstrap, frozen in the matrix (`[matrix.gate]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapSpec {
    pub seed: u64,
    pub resamples: usize,
}

/// The paired bootstrap interval on the detected-failure difference (SPEC "Gates"), treatment
/// minus control, over expected-fail items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairedBootstrap {
    pub seed: u64,
    pub resamples: usize,
    pub point: f64,
    pub lower: f64,
    pub upper: f64,
    pub insufficient_tail: bool,
}

/// Two arms of one model compared on the items both scored (repetition 1): the items that
/// flipped each way and, when the matrix froze a seed, the bootstrap interval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairedMetrics {
    /// `profile:model`.
    pub model: String,
    /// The control cell (arm A against C, arm C against a D arm).
    pub control: String,
    pub treatment: String,
    /// Items scored under both cells.
    pub items: usize,
    /// Expected-fail items the treatment detected and the control did not, and the reverse.
    pub detected_gained: usize,
    pub detected_lost: usize,
    /// Expected-pass items the treatment failed and the control did not, and the reverse.
    pub false_rejects_gained: usize,
    pub false_rejects_lost: usize,
    /// Families with an expected-fail item whose detection differs between the arms.
    pub differing_families: usize,
    pub flips: Vec<Flip>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<PairedBootstrap>,
    /// Why the bootstrap could not run, when a seed was frozen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap_note: Option<String>,
}

/// The metrics of a matrix: one entry per cell, in cell id order, and the paired comparisons the
/// cells allow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixMetrics {
    pub metrics_version: u32,
    pub policy: Policy,
    pub cells: Vec<CellMetrics>,
    #[serde(default)]
    pub paired: Vec<PairedMetrics>,
}

/// Metrics for every cell present in `rows`, under the frozen `policy`, with paired flips and no
/// bootstrap interval.
pub fn compute(rows: &[ItemRow], policy: &Policy) -> MatrixMetrics {
    compute_with(rows, policy, None)
}

/// [`compute`], with the paired bootstrap under `bootstrap` when given.
pub fn compute_with(
    rows: &[ItemRow],
    policy: &Policy,
    bootstrap: Option<BootstrapSpec>,
) -> MatrixMetrics {
    let mut by_cell: BTreeMap<&str, Vec<&ItemRow>> = BTreeMap::new();
    for r in rows {
        by_cell.entry(r.cell.as_str()).or_default().push(r);
    }
    let paired = paired_metrics(&by_cell, bootstrap);
    MatrixMetrics {
        metrics_version: 1,
        policy: *policy,
        cells: by_cell
            .into_iter()
            .map(|(cell, rows)| cell_metrics(cell, &rows, policy))
            .collect(),
        paired,
    }
}

/// Splits a cell id `profile:model/arm` into the model and the arm.
fn split_cell(cell: &str) -> Option<(&str, &str)> {
    cell.rsplit_once('/')
}

/// Every control/treatment pair the cells allow: A against C, and C against each D arm, per
/// model.
fn pairs(by_cell: &BTreeMap<&str, Vec<&ItemRow>>) -> Vec<(String, String, String)> {
    let mut arms: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for cell in by_cell.keys() {
        if let Some((model, arm)) = split_cell(cell) {
            arms.entry(model).or_default().push(arm);
        }
    }
    let mut out = Vec::new();
    for (model, arms) in arms {
        let has = |a: &str| arms.contains(&a);
        if has("A") && has("C") {
            out.push((
                model.to_string(),
                format!("{model}/A"),
                format!("{model}/C"),
            ));
        }
        if has("C") {
            for arm in arms.iter().filter(|a| a.starts_with('D')) {
                out.push((
                    model.to_string(),
                    format!("{model}/C"),
                    format!("{model}/{arm}"),
                ));
            }
        }
    }
    out
}

fn paired_metrics(
    by_cell: &BTreeMap<&str, Vec<&ItemRow>>,
    bootstrap: Option<BootstrapSpec>,
) -> Vec<PairedMetrics> {
    let headline = |cell: &str| -> BTreeMap<(String, String), &ItemRow> {
        by_cell
            .get(cell)
            .map(|rows| {
                rows.iter()
                    .filter(|r| r.repetition == 1)
                    .map(|r| ((r.set.clone(), r.item.clone()), *r))
                    .collect()
            })
            .unwrap_or_default()
    };
    pairs(by_cell)
        .into_iter()
        .map(|(model, control, treatment)| {
            let c_rows = headline(&control);
            let x_rows = headline(&treatment);
            let mut p = PairedMetrics {
                model,
                control,
                treatment,
                items: 0,
                detected_gained: 0,
                detected_lost: 0,
                false_rejects_gained: 0,
                false_rejects_lost: 0,
                differing_families: 0,
                flips: Vec::new(),
                bootstrap: None,
                bootstrap_note: None,
            };
            let mut paired_items = Vec::new();
            for (key, c) in &c_rows {
                let Some(x) = x_rows.get(key) else { continue };
                p.items += 1;
                let (c_fail, x_fail) = (c.computed == Computed::Fail, x.computed == Computed::Fail);
                match (c.expected, c_fail, x_fail) {
                    (Expected::Fail, false, true) => p.detected_gained += 1,
                    (Expected::Fail, true, false) => p.detected_lost += 1,
                    (Expected::Pass, false, true) => p.false_rejects_gained += 1,
                    (Expected::Pass, true, false) => p.false_rejects_lost += 1,
                    _ => {}
                }
                if c.computed != x.computed {
                    p.flips.push(Flip {
                        set: key.0.clone(),
                        item: key.1.clone(),
                        expected: c.expected,
                        from: c.computed,
                        to: x.computed,
                    });
                }
                paired_items.push(crate::bootstrap::PairedItem {
                    family: format!("{}/{}", key.0, c.family),
                    id: format!("{}/{}", key.0, key.1),
                    expected_fail: c.expected == Expected::Fail,
                    detected_c: c_fail,
                    detected_x: x_fail,
                });
            }
            p.differing_families = crate::bootstrap::canonical_families(&paired_items)
                .map(|f| f.iter().filter(|f| f.differs()).count())
                .unwrap_or(0);
            if let Some(spec) = bootstrap {
                match crate::bootstrap::paired_bootstrap(
                    &paired_items,
                    spec.seed,
                    spec.resamples,
                    1,
                ) {
                    Ok(b) => {
                        p.bootstrap = Some(PairedBootstrap {
                            seed: b.seed,
                            resamples: b.resamples,
                            point: b.point,
                            lower: b.lower,
                            upper: b.upper,
                            insufficient_tail: b.insufficient_tail,
                        });
                    }
                    Err(e) => p.bootstrap_note = Some(e.to_string()),
                }
            }
            p
        })
        .collect()
}

fn column(c: Computed) -> usize {
    match c {
        Computed::Pass => 0,
        Computed::Fail => 1,
        Computed::Unsure => 2,
        Computed::Error => 3,
    }
}

fn cell_metrics(cell: &str, rows: &[&ItemRow], policy: &Policy) -> CellMetrics {
    let headline: Vec<&ItemRow> = rows.iter().copied().filter(|r| r.repetition == 1).collect();
    let mut counts = Counts {
        items: headline.len(),
        ..Counts::default()
    };
    let mut confusion = Confusion::default();
    let mut localisation = Localisation::default();
    let mut costs = Vec::with_capacity(headline.len());
    let mut walls = Vec::with_capacity(headline.len());
    for r in &headline {
        let col = column(r.computed);
        match r.expected {
            Expected::Fail => {
                counts.expected_fail += 1;
                confusion.expected_fail[col] += 1;
                if r.computed == Computed::Fail {
                    counts.detected += 1;
                    if !r.labels.is_empty() {
                        localise(r, &mut localisation);
                    }
                } else if policy.passes(r.computed) {
                    counts.missed += 1;
                }
            }
            Expected::Pass => {
                counts.expected_pass += 1;
                confusion.expected_pass[col] += 1;
                if r.computed == Computed::Fail {
                    counts.false_rejects += 1;
                }
            }
        }
        match r.computed {
            Computed::Unsure => counts.unsure += 1,
            Computed::Error => counts.errors += 1,
            _ => {}
        }
        costs.push(r.cost_usd);
        walls.push(r.wall_ms);
    }
    let rates = Rates {
        missed: Proportion::of(counts.missed, counts.expected_fail),
        detected: Proportion::of(counts.detected, counts.expected_fail),
        false_rejects: Proportion::of(counts.false_rejects, counts.expected_pass),
        unsure: Proportion::of(counts.unsure, counts.items),
        errors: Proportion::of(counts.errors, counts.items),
    };
    // Agreement: every later repetition's verdict against the decision repetition's.
    let first: BTreeMap<(&str, &str), Computed> = headline
        .iter()
        .map(|r| ((r.set.as_str(), r.item.as_str()), r.computed))
        .collect();
    let mut count = 1;
    let (mut agree, mut later) = (0usize, 0usize);
    for r in rows.iter().filter(|r| r.repetition > 1) {
        count = count.max(r.repetition);
        if let Some(c) = first.get(&(r.set.as_str(), r.item.as_str())) {
            later += 1;
            if *c == r.computed {
                agree += 1;
            }
        }
    }
    CellMetrics {
        cell: cell.to_string(),
        counts,
        confusion,
        rates,
        localisation,
        cost: Cost {
            total_usd: costs.iter().sum(),
            median_usd: median_f64(&mut costs),
        },
        wall: Wall {
            total_ms: walls.iter().sum(),
            median_ms: median_u64(&mut walls),
        },
        repetitions: Repetitions {
            count,
            agreement: (later > 0).then(|| agree as f64 / later as f64),
        },
    }
}

/// The eligibility rules and the greedy one-to-one match for one detected failure.
fn localise(r: &ItemRow, l: &mut Localisation) {
    l.labelled_regions += r.labels.len();
    l.predictions += r.predictions.len();
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for (li, label) in r.labels.iter().enumerate() {
        for (pi, pred) in r.predictions.iter().enumerate() {
            if let Some(score) = eligible_score(label, pred) {
                pairs.push((score, li, pi));
            }
        }
    }
    // Descending score; ties by label then prediction order so the result is deterministic.
    pairs.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    let mut label_taken = vec![false; r.labels.len()];
    let mut pred_taken = vec![false; r.predictions.len()];
    let mut hits = 0;
    for (_, li, pi) in pairs {
        if !label_taken[li] && !pred_taken[pi] {
            label_taken[li] = true;
            pred_taken[pi] = true;
            hits += 1;
        }
    }
    l.hits += hits;
    l.spurious += r.predictions.len() - hits;
}

/// `Some(score)` when the pair is eligible and a hit: the same root image, intersecting
/// obligations, and IoU above the threshold for boxed regions, or the same kind for `whole_image`
/// and `absent`.
fn eligible_score(label: &Region, pred: &Region) -> Option<f64> {
    if label.image != pred.image
        || !label
            .obligation_ids
            .iter()
            .any(|id| pred.obligation_ids.contains(id))
    {
        return None;
    }
    match (label.kind, pred.kind) {
        (RegionKind::WholeImage, RegionKind::WholeImage)
        | (RegionKind::Absent, RegionKind::Absent) => Some(1.0),
        (RegionKind::Bbox | RegionKind::Polygon, RegionKind::Bbox | RegionKind::Polygon) => {
            let v = iou(label.boxed()?, pred.boxed()?);
            (v > HIT_IOU).then_some(v)
        }
        _ => None,
    }
}

/// Intersection over union of two `[left, top, right, bottom]` boxes.
pub fn iou(a: [f64; 4], b: [f64; 4]) -> f64 {
    let area = |r: [f64; 4]| ((r[2] - r[0]).max(0.0)) * ((r[3] - r[1]).max(0.0));
    let inter = [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ];
    let i = area(inter);
    let u = area(a) + area(b) - i;
    if u <= 0.0 { 0.0 } else { i / u }
}

fn median_f64(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

fn median_u64(v: &mut [u64]) -> u64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_iou_is_exact_on_a_hand_case() {
        let v = iou([0.1, 0.1, 0.3, 0.3], [0.12, 0.12, 0.32, 0.32]);
        assert!((v - 0.0324 / 0.0476).abs() < 1e-12);
        assert_eq!(iou([0.0, 0.0, 0.1, 0.1], [0.5, 0.5, 0.6, 0.6]), 0.0);
    }

    #[test]
    fn stats_medians_take_the_middle_or_the_mean_of_the_middle_two() {
        assert_eq!(median_f64(&mut [3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median_f64(&mut [4.0, 1.0, 3.0, 2.0]), 2.5);
        assert_eq!(median_u64(&mut [50, 500, 100, 400, 200, 300]), 250);
        assert_eq!(median_u64(&mut []), 0);
    }
}
