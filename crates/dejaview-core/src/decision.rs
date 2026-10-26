//! Decisions: what `check` returns. A decision is one or two independent runs of the state
//! machine over the same resolved job, combined by the spec's table (SPEC "Decisions and runs"),
//! and the consumer policy that maps the raw verdict to an exit code.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::aggregate::{Confidence, Disposition, Verdict};
use crate::checklist::iou;
use crate::config::{PolicyConfig, PolicyOutcome};
use crate::frames::Location;

/// What one run contributes to the decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSummary {
    /// `None` when the run ended in an error.
    pub verdict: Option<Verdict>,
    pub items: Vec<DecisionItem>,
}

/// One dispositioned checklist item, with the code-owned fields matching needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionItem {
    pub id: String,
    pub disposition: Disposition,
    pub confidence: Confidence,
    pub allowed_by: Option<String>,
    pub root_image: Option<String>,
    pub instance_ids: Vec<String>,
    /// Evidence items only: the region in root-frame coordinates.
    pub root_location: Option<Location>,
}

impl DecisionItem {
    fn is_evidence(&self) -> bool {
        self.id.starts_with("chk:ev:")
    }
    fn same_answer(&self, other: &DecisionItem) -> bool {
        self.disposition == other.disposition
            && self.confidence == other.confidence
            && self.allowed_by == other.allowed_by
    }
}

/// The combined result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// The combined verdict, `None` when the decision is an error.
    pub verdict: Option<Verdict>,
    pub error: bool,
    /// For two runs that both passed: whether they agreed. `None` otherwise.
    pub agree: Option<bool>,
    pub disagreements: Vec<String>,
}

/// IoU floor for matching evidence items across runs.
pub const MATCH_IOU: f64 = 0.5;

/// One run is the decision.
pub fn single(run: &RunSummary) -> Decision {
    Decision {
        verdict: run.verdict,
        error: run.verdict.is_none(),
        agree: None,
        disagreements: Vec::new(),
    }
}

/// The two-run table:
///
/// |            | pass            | unsure | fail | error |
/// |------------|-----------------|--------|------|-------|
/// | **pass**   | pass if agree, else unsure | unsure | fail | error |
/// | **unsure** | unsure          | unsure | fail | error |
/// | **fail**   | fail            | fail   | fail | fail  |
/// | **error**  | error           | error  | fail | error |
pub fn combine(a: &RunSummary, b: &RunSummary) -> Decision {
    use Verdict::*;
    match (a.verdict, b.verdict) {
        (Some(Fail), _) | (_, Some(Fail)) => Decision {
            verdict: Some(Fail),
            error: false,
            agree: None,
            disagreements: vec![],
        },
        (None, _) | (_, None) => Decision {
            verdict: None,
            error: true,
            agree: None,
            disagreements: vec![],
        },
        (Some(Unsure), _) | (_, Some(Unsure)) => Decision {
            verdict: Some(Unsure),
            error: false,
            agree: None,
            disagreements: vec![],
        },
        (Some(Pass), Some(Pass)) => {
            let disagreements = disagreements(a, b);
            let agree = disagreements.is_empty();
            Decision {
                verdict: Some(if agree { Pass } else { Unsure }),
                error: false,
                agree: Some(agree),
                disagreements,
            }
        }
    }
}

/// Static items compare by id; evidence items are matched one-to-one (same root image, identical
/// dependency sets, IoU above the floor; sorted by descending IoU, then run 1 order, then run 2
/// order; greedy). An unmatched evidence item disagrees unless it was `held`.
pub fn disagreements(a: &RunSummary, b: &RunSummary) -> Vec<String> {
    let mut out = Vec::new();
    for ia in a.items.iter().filter(|i| !i.is_evidence()) {
        match b.items.iter().find(|ib| ib.id == ia.id) {
            Some(ib) if ia.same_answer(ib) => {}
            Some(_) => out.push(format!("{}: dispositions differ", ia.id)),
            None => out.push(format!("{}: missing in run 2", ia.id)),
        }
    }
    for ib in b.items.iter().filter(|i| !i.is_evidence()) {
        if !a.items.iter().any(|ia| ia.id == ib.id) {
            out.push(format!("{}: missing in run 1", ib.id));
        }
    }

    let ea: Vec<(usize, &DecisionItem)> = a
        .items
        .iter()
        .filter(|i| i.is_evidence())
        .enumerate()
        .collect();
    let eb: Vec<(usize, &DecisionItem)> = b
        .items
        .iter()
        .filter(|i| i.is_evidence())
        .enumerate()
        .collect();
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for (i, x) in &ea {
        for (j, y) in &eb {
            let eligible = x.root_image == y.root_image && x.instance_ids == y.instance_ids;
            if !eligible {
                continue;
            }
            if let (Some(lx), Some(ly)) = (&x.root_location, &y.root_location) {
                let v = iou(lx, ly);
                if v > MATCH_IOU {
                    pairs.push((v, *i, *j));
                }
            }
        }
    }
    pairs.sort_by(|p, q| {
        q.0.partial_cmp(&p.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(p.1.cmp(&q.1))
            .then(p.2.cmp(&q.2))
    });
    let mut used_a = vec![false; ea.len()];
    let mut used_b = vec![false; eb.len()];
    for (_, i, j) in pairs {
        if used_a[i] || used_b[j] {
            continue;
        }
        used_a[i] = true;
        used_b[j] = true;
        let (x, y) = (ea[i].1, eb[j].1);
        if !x.same_answer(y) {
            out.push(format!("{} vs {}: dispositions differ", x.id, y.id));
        }
    }
    for (k, (_, x)) in ea.iter().enumerate() {
        if !used_a[k] && x.disposition != Disposition::Held {
            out.push(format!("{}: unmatched in run 2 and not held", x.id));
        }
    }
    for (k, (_, y)) in eb.iter().enumerate() {
        if !used_b[k] && y.disposition != Disposition::Held {
            out.push(format!("{}: unmatched in run 1 and not held", y.id));
        }
    }
    out
}

/// Exit code for a verdict (or error) after the consumer policy. The report keeps the raw verdict.
pub fn exit_code(verdict: Option<Verdict>, policy: &PolicyConfig) -> i32 {
    match verdict {
        Some(Verdict::Pass) => 0,
        Some(Verdict::Fail) => 1,
        Some(Verdict::Unsure) => match policy.on_unsure {
            PolicyOutcome::Pass => 0,
            PolicyOutcome::Fail => 1,
            PolicyOutcome::Review => 2,
        },
        None => match policy.on_error {
            PolicyOutcome::Pass => 0,
            PolicyOutcome::Fail => 1,
            PolicyOutcome::Review => 3,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::LocationKind;

    fn run(verdict: Option<Verdict>, items: Vec<DecisionItem>) -> RunSummary {
        RunSummary { verdict, items }
    }

    fn item(id: &str, d: Disposition) -> DecisionItem {
        DecisionItem {
            id: id.into(),
            disposition: d,
            confidence: Confidence::Confident,
            allowed_by: None,
            root_image: Some("candidate".into()),
            instance_ids: vec!["SF1@candidate".into()],
            root_location: None,
        }
    }

    fn ev(id: &str, d: Disposition, b: [f64; 4]) -> DecisionItem {
        let mut i = item(id, d);
        i.root_location = Some(Location {
            frame: "raw:candidate".into(),
            kind: LocationKind::Bbox,
            bbox: Some(b),
            polygon: None,
            yaw: None,
            pitch: None,
            fov: None,
        });
        i
    }

    #[test]
    fn the_sixteen_cell_table() {
        use Verdict::*;
        let v = [Some(Pass), Some(Unsure), Some(Fail), None];
        let expected: [[Option<Verdict>; 4]; 4] = [
            [Some(Pass), Some(Unsure), Some(Fail), None],
            [Some(Unsure), Some(Unsure), Some(Fail), None],
            [Some(Fail), Some(Fail), Some(Fail), Some(Fail)],
            [None, None, Some(Fail), None],
        ];
        for (i, a) in v.iter().enumerate() {
            for (j, b) in v.iter().enumerate() {
                let d = combine(
                    &run(*a, vec![item("chk:ob:SF1@candidate", Disposition::Held)]),
                    &run(*b, vec![item("chk:ob:SF1@candidate", Disposition::Held)]),
                );
                assert_eq!(d.verdict, expected[i][j], "{a:?} x {b:?}");
                assert_eq!(
                    d.error,
                    expected[i][j].is_none(),
                    "{a:?} x {b:?} error flag"
                );
            }
        }
    }

    #[test]
    fn two_passes_that_disagree_on_a_static_item_are_unsure() {
        let a = run(
            Some(Verdict::Pass),
            vec![item("chk:ob:SF1@candidate", Disposition::Held)],
        );
        let mut hidden = item("chk:ob:SF1@candidate", Disposition::Hidden);
        hidden.confidence = Confidence::Confident;
        let b = run(Some(Verdict::Pass), vec![hidden]);
        let d = combine(&a, &b);
        assert_eq!(d.verdict, Some(Verdict::Unsure));
        assert_eq!(d.agree, Some(false));
        assert_eq!(
            d.disagreements,
            ["chk:ob:SF1@candidate: dispositions differ"]
        );
    }

    #[test]
    fn allowed_by_identity_is_part_of_agreement() {
        let mut x = item("chk:ob:SF1@candidate", Disposition::AllowedChange);
        x.allowed_by = Some("RQ1".into());
        let mut y = x.clone();
        y.allowed_by = Some("RQ2".into());
        assert!(
            !disagreements(
                &run(Some(Verdict::Pass), vec![x]),
                &run(Some(Verdict::Pass), vec![y])
            )
            .is_empty()
        );
    }

    #[test]
    fn evidence_matching_requires_root_image_and_dependencies_and_iou() {
        let a = run(
            Some(Verdict::Pass),
            vec![ev("chk:ev:1:0", Disposition::Held, [0.1, 0.1, 0.5, 0.5])],
        );
        let b = run(
            Some(Verdict::Pass),
            vec![ev("chk:ev:2:0", Disposition::Held, [0.1, 0.1, 0.5, 0.5])],
        );
        assert!(
            disagreements(&a, &b).is_empty(),
            "identical boxes match and agree"
        );
        let mut other_deps = ev("chk:ev:2:0", Disposition::Held, [0.1, 0.1, 0.5, 0.5]);
        other_deps.instance_ids = vec!["SF2@candidate".into()];
        let b2 = run(Some(Verdict::Pass), vec![other_deps]);
        assert!(
            disagreements(&a, &b2).is_empty(),
            "unmatched but held on both sides: no disagreement"
        );
        let mut a_viol = ev("chk:ev:1:0", Disposition::CantTell, [0.1, 0.1, 0.5, 0.5]);
        a_viol.confidence = Confidence::Tentative;
        let a2 = run(Some(Verdict::Pass), vec![a_viol]);
        assert_eq!(
            disagreements(&a2, &b2),
            ["chk:ev:1:0: unmatched in run 2 and not held"]
        );
        let far = run(
            Some(Verdict::Pass),
            vec![ev("chk:ev:2:0", Disposition::Held, [0.6, 0.6, 0.9, 0.9])],
        );
        assert!(disagreements(&a, &far).is_empty(), "no overlap, both held");
    }

    #[test]
    fn greedy_matching_prefers_higher_iou_then_run_order() {
        let a = run(
            Some(Verdict::Pass),
            vec![
                ev("chk:ev:1:0", Disposition::Held, [0.0, 0.0, 0.5, 0.5]),
                ev("chk:ev:1:1", Disposition::CantTell, [0.0, 0.0, 0.45, 0.5]),
            ],
        );
        let b = run(
            Some(Verdict::Pass),
            vec![ev(
                "chk:ev:2:0",
                Disposition::CantTell,
                [0.0, 0.0, 0.45, 0.5],
            )],
        );
        // ev:1:1 and ev:2:0 are identical (IoU 1) and both cant_tell: they match and agree;
        // ev:1:0 is left unmatched but held.
        assert!(disagreements(&a, &b).is_empty());
    }

    #[test]
    fn policy_maps_only_unsure_and_error() {
        let review = PolicyConfig::default();
        assert_eq!(exit_code(Some(Verdict::Pass), &review), 0);
        assert_eq!(exit_code(Some(Verdict::Fail), &review), 1);
        assert_eq!(exit_code(Some(Verdict::Unsure), &review), 2);
        assert_eq!(exit_code(None, &review), 3);
        let lenient = PolicyConfig {
            on_unsure: PolicyOutcome::Pass,
            on_error: PolicyOutcome::Fail,
        };
        assert_eq!(exit_code(Some(Verdict::Unsure), &lenient), 0);
        assert_eq!(exit_code(None, &lenient), 1);
        assert_eq!(
            exit_code(Some(Verdict::Fail), &lenient),
            1,
            "fail is never softened"
        );
    }
}
