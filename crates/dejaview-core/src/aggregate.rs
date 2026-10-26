//! Aggregation: the verdict computed in code from dispositions and the instances they depend on.
//! This is the spec's table under "Aggregation", one row per mandatory instance, strictest row
//! kept. The model's text never reaches these functions.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::obligations::Instance;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Violation,
    Held,
    AllowedChange,
    Hidden,
    CantTell,
    /// Findings only: an observation under an `observe` obligation.
    Observed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Confident,
    Tentative,
}

/// One aggregation row. Ordered so that `max` is the strictest.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Row {
    Recorded,
    Unsure,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    Unsure,
}

impl Verdict {
    /// The raw exit code before `[policy]` is applied.
    pub fn exit_code(self) -> i32 {
        match self {
            Verdict::Pass => 0,
            Verdict::Fail => 1,
            Verdict::Unsure => 2,
        }
    }
}

/// One aggregation row for one mandatory instance. Validation guarantees every disposition that
/// reaches here depends on mandatory instances and that an `allowed_change` cites an applicable
/// exemption.
pub fn row(inst: &Instance, d: Disposition, c: Confidence) -> Row {
    use Confidence::*;
    use Disposition::*;
    match (d, c) {
        (Violation, Confident) => Row::Fail,
        (Violation, Tentative) => Row::Unsure,
        (CantTell, _) if inst.matters => Row::Unsure,
        (Hidden, _) if inst.matters && !inst.hidden_ok => Row::Unsure,
        (Held, Tentative) if inst.matters => Row::Unsure,
        (AllowedChange, Tentative) if inst.matters => Row::Unsure,
        _ => Row::Recorded,
    }
}

/// The strictest of several rows; `Recorded` when there are none.
pub fn strictest(rows: impl IntoIterator<Item = Row>) -> Row {
    rows.into_iter().max().unwrap_or(Row::Recorded)
}

/// The verdict from every row of a run.
pub fn verdict(rows: &[Row]) -> Verdict {
    match rows.iter().max() {
        Some(Row::Fail) => Verdict::Fail,
        Some(Row::Unsure) => Verdict::Unsure,
        _ => Verdict::Pass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obligations::Kind;

    fn inst(matters: bool, hidden_ok: bool) -> Instance {
        Instance {
            obligation_id: "X1".into(),
            target: "candidate".into(),
            kind: Kind::MustHold,
            matters,
            hidden_ok,
            text: String::new(),
            evidence: vec![],
            evidence_kinds: vec![],
            provenance: "test".into(),
        }
    }

    /// Every disposition x confidence x (matters, hidden_ok) against the spec table.
    #[test]
    fn aggregation_table_matches_the_spec_row_by_row() {
        use Confidence::*;
        use Disposition::*;
        // The spec's table, transcribed: every disposition, confidence and flag combination.
        fn spec_row(d: Disposition, c: Confidence, matters: bool, hidden_ok: bool) -> Row {
            match (d, c) {
                (Violation, Confident) => Row::Fail,
                (Violation, Tentative) => Row::Unsure,
                (CantTell, _) => {
                    if matters {
                        Row::Unsure
                    } else {
                        Row::Recorded
                    }
                }
                (Hidden, _) => {
                    if matters && !hidden_ok {
                        Row::Unsure
                    } else {
                        Row::Recorded
                    }
                }
                (Held, Tentative) | (AllowedChange, Tentative) => {
                    if matters {
                        Row::Unsure
                    } else {
                        Row::Recorded
                    }
                }
                (Held, Confident) | (AllowedChange, Confident) | (Observed, _) => Row::Recorded,
            }
        }
        let mut cells = 0;
        for d in [Violation, Held, AllowedChange, Hidden, CantTell, Observed] {
            for c in [Confident, Tentative] {
                for matters in [true, false] {
                    for hidden_ok in [true, false] {
                        cells += 1;
                        assert_eq!(
                            row(&inst(matters, hidden_ok), d, c),
                            spec_row(d, c, matters, hidden_ok),
                            "{d:?} {c:?} matters={matters} hidden_ok={hidden_ok}"
                        );
                    }
                }
            }
        }
        assert_eq!(cells, 48, "the whole domain");
    }

    #[test]
    fn strictest_row_wins_and_verdict_follows() {
        assert_eq!(
            strictest([Row::Recorded, Row::Unsure, Row::Recorded]),
            Row::Unsure
        );
        assert_eq!(strictest([Row::Unsure, Row::Fail]), Row::Fail);
        assert_eq!(strictest(std::iter::empty()), Row::Recorded);
        assert_eq!(verdict(&[Row::Recorded]), Verdict::Pass);
        assert_eq!(verdict(&[Row::Recorded, Row::Unsure]), Verdict::Unsure);
        assert_eq!(verdict(&[Row::Unsure, Row::Fail]), Verdict::Fail);
        assert_eq!(verdict(&[]), Verdict::Pass);
    }

    #[test]
    fn exit_codes_follow_the_spec() {
        assert_eq!(Verdict::Pass.exit_code(), 0);
        assert_eq!(Verdict::Fail.exit_code(), 1);
        assert_eq!(Verdict::Unsure.exit_code(), 2);
    }
}
