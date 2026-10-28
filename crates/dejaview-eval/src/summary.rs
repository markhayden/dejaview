//! The evaluation summary: item rows gathered from an evaluation directory and its sets' labels,
//! the metrics computed from them, `metrics.json` and the Markdown `report.md`. The runner writes
//! both at the end of every run; `dejaview eval report` regenerates them from the saved results,
//! byte for byte, because everything they hold is derived from `summary.json`, the decision
//! reports and the labels.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use dejaview_core::config::PolicyConfig;
use dejaview_core::frames::LocationKind;
use dejaview_core::submission::FindingDisposition;

use crate::gate_rules::{Computed, Policy};
use crate::metrics::{self, CellMetrics, ItemRow, MatrixMetrics, Region, RegionKind};
use crate::runner::{DecisionResult, SUMMARY_FILE, Summary};
use crate::sets::{Labels, Set};
use crate::{Error, Result};

pub const METRICS_FILE: &str = "metrics.json";
pub const REPORT_FILE: &str = "report.md";

fn io(path: &Path, e: std::io::Error) -> Error {
    Error::Runner(format!("{}: {e}", path.display()))
}

/// The frozen policy under which the metrics are computed: the `[policy]` section of the
/// evaluation's config, as every decision report records it.
pub fn policy_of(policy: &PolicyConfig) -> Policy {
    Policy {
        on_unsure: policy.on_unsure,
        on_error: policy.on_error,
    }
}

/// The root image id of a label's frame: labels name the raw frame (`raw:candidate`).
fn image_of(frame: &str) -> &str {
    frame.strip_prefix("raw:").unwrap_or(frame)
}

fn computed_of(d: &DecisionResult) -> Computed {
    match d.verdict.as_deref() {
        Some("pass") => Computed::Pass,
        Some("fail") => Computed::Fail,
        Some("unsure") => Computed::Unsure,
        _ => Computed::Error,
    }
}

/// The violations the decision submitted, in root coordinates, from the decision's report: the
/// first run that reached `fail`, or the first run when none did. A finding whose location did
/// not resolve to a root image keeps the frame it named, so it can match no label and counts as
/// spurious.
fn predictions_of(report: &dejaview_core::report::Report) -> Vec<Region> {
    let run = report
        .runs
        .iter()
        .find(|r| r.verdict == Some(dejaview_core::aggregate::Verdict::Fail))
        .or_else(|| report.runs.first());
    let Some(run) = run else {
        return Vec::new();
    };
    run.findings
        .iter()
        .filter(|f| f.disposition == FindingDisposition::Violation)
        .map(|f| {
            let (image, kind, bbox, polygon) = match &f.root {
                Some(root) => (
                    root.image.clone(),
                    root.kind,
                    root.bbox,
                    root.polygon.clone(),
                ),
                None => (
                    f.location.frame.clone(),
                    f.location.kind,
                    f.location.bbox,
                    f.location.polygon.clone(),
                ),
            };
            Region {
                image,
                kind: match kind {
                    LocationKind::Bbox => RegionKind::Bbox,
                    LocationKind::Polygon => RegionKind::Polygon,
                    LocationKind::WholeImage => RegionKind::WholeImage,
                    LocationKind::Absent => RegionKind::Absent,
                    // A view has no root geometry the labels can carry; it matches nothing.
                    LocationKind::View => RegionKind::Bbox,
                },
                bbox: if kind == LocationKind::View {
                    None
                } else {
                    bbox
                },
                polygon,
                obligation_ids: f.obligation_ids.clone(),
            }
        })
        .collect()
}

/// The labels of every set that has them, by set id.
pub fn labels_of(sets: &[Set]) -> BTreeMap<&str, &Labels> {
    sets.iter()
        .filter_map(|s| s.labels.as_ref().map(|l| (s.manifest.set.id.as_str(), l)))
        .collect()
}

/// One row per decision of a labelled item, with the policy the reports record. Items without a
/// label are not scored; decisions whose report is missing contribute no predictions.
pub fn rows(
    eval_dir: &Path,
    labels: &BTreeMap<&str, &Labels>,
    summary: &Summary,
) -> Result<(Vec<ItemRow>, Policy)> {
    let labels: BTreeMap<(&str, &str), &crate::sets::Label> = labels
        .iter()
        .flat_map(|(set, l)| l.label.iter().map(move |l| ((*set, l.item.as_str()), l)))
        .collect();
    let mut policy = None;
    let mut out = Vec::new();
    for d in &summary.decisions {
        let Some(label) = labels.get(&(d.set.as_str(), d.item.as_str())) else {
            continue;
        };
        let report_path = eval_dir.join(&d.dir).join("report.json");
        let report: Option<dejaview_core::report::Report> =
            match std::fs::read_to_string(&report_path) {
                Ok(text) => Some(
                    serde_json::from_str(&text)
                        .map_err(|e| Error::Runner(format!("{}: {e}", report_path.display())))?,
                ),
                Err(_) => None,
            };
        if let Some(r) = &report {
            let p = policy_of(&r.policy);
            if let Some(seen) = policy
                && seen != p
            {
                return Err(Error::Runner(format!(
                    "{}: the policy differs from the other decisions'",
                    d.id
                )));
            }
            policy = Some(p);
        }
        out.push(ItemRow {
            set: d.set.clone(),
            item: d.item.clone(),
            cell: d.cell.clone(),
            repetition: d.repetition,
            expected: label.expected,
            computed: computed_of(d),
            cost_usd: d.cost_usd,
            wall_ms: d.wall_ms,
            labels: label
                .regions
                .iter()
                .map(|r| Region {
                    image: image_of(&r.frame).to_string(),
                    kind: r.kind,
                    bbox: r.bbox,
                    polygon: r.polygon.clone(),
                    obligation_ids: r.obligation_ids.clone(),
                })
                .collect(),
            predictions: report.as_ref().map(predictions_of).unwrap_or_default(),
        });
    }
    Ok((
        out,
        policy.unwrap_or_else(|| policy_of(&PolicyConfig::default())),
    ))
}

/// Compute the metrics of an evaluation directory and write `metrics.json` and `report.md`.
pub fn write(eval_dir: &Path, sets: &[Set], summary: &Summary) -> Result<MatrixMetrics> {
    let (rows, policy) = rows(eval_dir, &labels_of(sets), summary)?;
    let m = metrics::compute(&rows, &policy);
    let json = serde_json::to_string_pretty(&m).map_err(|e| Error::Runner(e.to_string()))?;
    std::fs::write(eval_dir.join(METRICS_FILE), json + "\n").map_err(|e| io(eval_dir, e))?;
    std::fs::write(eval_dir.join(REPORT_FILE), render(summary, &m)).map_err(|e| io(eval_dir, e))?;
    Ok(m)
}

fn load_summary(eval_dir: &Path) -> Result<Summary> {
    let path = eval_dir.join(SUMMARY_FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| io(&path, e))?;
    serde_json::from_str(&text).map_err(|e| Error::Runner(format!("{}: {e}", path.display())))
}

fn load_sets(evalsets_root: &Path, summary: &Summary) -> Result<Vec<Set>> {
    let mut ids: Vec<&str> = summary.decisions.iter().map(|d| d.set.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.iter()
        .map(|id| Set::load(&evalsets_root.join(id)))
        .collect()
}

/// Load `summary.json` and the sets it names from `evalsets_root`, then [`write`].
pub fn regenerate(eval_dir: &Path, evalsets_root: &Path) -> Result<MatrixMetrics> {
    let summary = load_summary(eval_dir)?;
    let sets = load_sets(evalsets_root, &summary)?;
    write(eval_dir, &sets, &summary)
}

/// The Markdown report over the named sets only (every set when `only` is empty), without
/// touching the directory: how the results document reports development and adversarial sets
/// separately.
pub fn render_subset(eval_dir: &Path, evalsets_root: &Path, only: &[String]) -> Result<String> {
    let mut summary = load_summary(eval_dir)?;
    if !only.is_empty() {
        for id in only {
            if !summary.decisions.iter().any(|d| d.set == *id) {
                return Err(Error::Runner(format!(
                    "the summary has no decisions of set {id}"
                )));
            }
        }
        summary.decisions.retain(|d| only.contains(&d.set));
    }
    let sets = load_sets(evalsets_root, &summary)?;
    let (rows, policy) = rows(eval_dir, &labels_of(&sets), &summary)?;
    Ok(render(&summary, &metrics::compute(&rows, &policy)))
}

fn pct(p: &metrics::Proportion) -> String {
    format!(
        "{} / {} | {:.3} | {:.3} to {:.3}",
        p.count, p.total, p.value, p.low, p.high
    )
}

/// The Markdown report. Deterministic in its inputs: cells in id order, fixed number formats.
pub fn render(summary: &Summary, m: &MatrixMetrics) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# Evaluation report: {}", summary.matrix);
    let _ = writeln!(s);
    let labelled: usize = m.cells.iter().map(|c| c.counts.items).sum();
    let _ = writeln!(
        s,
        "Matrix identity `{}`, descriptor `{}`. {} decisions; {} labelled items scored across {} cell{}; ${:.4} of ${:.2} spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `{:?}` and error to `{:?}`.",
        summary.matrix_identity,
        summary.descriptor_sha256,
        summary.decisions.len(),
        labelled,
        m.cells.len(),
        if m.cells.len() == 1 { "" } else { "s" },
        summary.spent_usd,
        summary.cap_usd,
        m.policy.on_unsure,
        m.policy.on_error,
    );
    for c in &m.cells {
        render_cell(&mut s, c);
    }
    s
}

fn render_cell(s: &mut String, c: &CellMetrics) {
    let _ = writeln!(s);
    let _ = writeln!(s, "## {}", c.cell);
    let _ = writeln!(s);
    let _ = writeln!(s, "| Metric | Count | Rate | 95% Wilson |");
    let _ = writeln!(s, "|---|---|---|---|");
    let _ = writeln!(s, "| Detected failures | {} |", pct(&c.rates.detected));
    let _ = writeln!(s, "| Missed failures | {} |", pct(&c.rates.missed));
    let _ = writeln!(s, "| False rejects | {} |", pct(&c.rates.false_rejects));
    let _ = writeln!(s, "| Unsure | {} |", pct(&c.rates.unsure));
    let _ = writeln!(s, "| Errors | {} |", pct(&c.rates.errors));
    let _ = writeln!(s);
    let _ = writeln!(s, "Confusion, expected by row and computed by column:");
    let _ = writeln!(s);
    let _ = writeln!(s, "| Expected | pass | fail | unsure | error |");
    let _ = writeln!(s, "|---|---|---|---|---|");
    let row = |name: &str, r: &[usize; 4]| {
        format!("| {name} | {} | {} | {} | {} |", r[0], r[1], r[2], r[3])
    };
    let _ = writeln!(s, "{}", row("fail", &c.confusion.expected_fail));
    let _ = writeln!(s, "{}", row("pass", &c.confusion.expected_pass));
    let _ = writeln!(s);
    let l = &c.localisation;
    let _ = writeln!(
        s,
        "Localisation over detected failures with labelled regions: {} hits of {} labelled regions, {} spurious of {} predictions.",
        l.hits, l.labelled_regions, l.spurious, l.predictions
    );
    let _ = writeln!(
        s,
        "Cost ${:.4} total, ${:.4} median per item. Wall {} ms total, {} ms median.",
        c.cost.total_usd, c.cost.median_usd, c.wall.total_ms, c.wall.median_ms
    );
    match c.repetitions.agreement {
        Some(a) => {
            let _ = writeln!(
                s,
                "Repetitions {}: later repetitions agree with repetition 1 on {:.3} of items.",
                c.repetitions.count, a
            );
        }
        None => {
            let _ = writeln!(s, "Repetitions 1.");
        }
    }
}
