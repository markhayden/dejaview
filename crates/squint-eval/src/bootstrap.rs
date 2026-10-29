//! The paired bootstrap (SPEC "Gates", ADR 0016). The estimand is the item-weighted paired
//! difference in the proportion of expected-fail items detected, arm X minus arm C. Families with
//! at least one expected-fail item are resampled with replacement, every paired item of a selected
//! family kept; the interval is formed from percentiles by the Hyndman and Fan type 7 quantile with
//! a recorded seed and Bonferroni-adjusted percentiles for `m` hypotheses.
//!
//! Reproducibility is the contract: families sorted by id, items sorted by id within a family,
//! ChaCha20 seeded from a 64-bit unsigned decimal, one `random_range(0..families)` draw per
//! selected family, in order. `rand` and `rand_chacha` are pinned exactly in the workspace.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// One labelled item under both arms of a paired comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedItem {
    pub family: String,
    pub id: String,
    pub expected_fail: bool,
    /// The detection indicator per arm: `computed_verdict == fail`.
    pub detected_c: bool,
    pub detected_x: bool,
}

/// A family in canonical order: its id and its items sorted by id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family {
    pub id: String,
    pub items: Vec<PairedItem>,
}

impl Family {
    fn expected_fails(&self) -> usize {
        self.items.iter().filter(|i| i.expected_fail).count()
    }

    fn detected_difference(&self) -> i64 {
        self.items
            .iter()
            .filter(|i| i.expected_fail)
            .map(|i| i64::from(i.detected_x) - i64::from(i.detected_c))
            .sum()
    }

    /// Whether an expected-fail item's detection indicator differs between the arms.
    pub fn differs(&self) -> bool {
        self.items
            .iter()
            .any(|i| i.expected_fail && i.detected_c != i.detected_x)
    }
}

/// Families with at least one expected-fail item, sorted by id, items sorted by id. Duplicate item
/// ids within a family are an error.
pub fn canonical_families(items: &[PairedItem]) -> Result<Vec<Family>> {
    let mut families: Vec<Family> = Vec::new();
    for item in items {
        match families.iter_mut().find(|f| f.id == item.family) {
            Some(f) => f.items.push(item.clone()),
            None => families.push(Family {
                id: item.family.clone(),
                items: vec![item.clone()],
            }),
        }
    }
    families.sort_by(|a, b| a.id.cmp(&b.id));
    for f in &mut families {
        f.items.sort_by(|a, b| a.id.cmp(&b.id));
        if f.items.windows(2).any(|w| w[0].id == w[1].id) {
            return Err(Error::Stats(format!(
                "family {:?} has a duplicate item id",
                f.id
            )));
        }
    }
    Ok(families
        .into_iter()
        .filter(|f| f.expected_fails() > 0)
        .collect())
}

/// The estimand over a selection of families (indices into `families`, with multiplicity).
pub fn estimand(families: &[Family], picks: &[usize]) -> f64 {
    let (diff, fails) = picks.iter().fold((0i64, 0usize), |(d, n), &i| {
        (
            d + families[i].detected_difference(),
            n + families[i].expected_fails(),
        )
    });
    if fails == 0 {
        0.0
    } else {
        diff as f64 / fails as f64
    }
}

/// The resample indices for a seed: `resamples` draws of `families` indices each.
pub fn resample_indices(seed: u64, families: usize, resamples: usize) -> Vec<Vec<usize>> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    (0..resamples)
        .map(|_| {
            (0..families)
                .map(|_| rng.random_range(0..families))
                .collect()
        })
        .collect()
}

/// Hyndman and Fan type 7: linear interpolation between order statistics, `h = (n - 1) p`.
pub fn quantile_type7(sorted: &[f64], p: f64) -> f64 {
    match sorted.len() {
        0 => f64::NAN,
        1 => sorted[0],
        n => {
            let h = (n - 1) as f64 * p.clamp(0.0, 1.0);
            let lo = h.floor() as usize;
            let hi = (lo + 1).min(n - 1);
            sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
        }
    }
}

/// Fewer resamples than this below the lower percentile cannot resolve the tail.
pub const MIN_TAIL_RESAMPLES: f64 = 20.0;

/// What the bootstrap produced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bootstrap {
    pub seed: u64,
    pub resamples: usize,
    pub hypotheses: usize,
    pub families: usize,
    /// Families with an expected-fail item whose detection indicator differs between arms.
    pub differing_families: usize,
    /// The estimand on the full sample.
    pub point: f64,
    pub lower_percentile: f64,
    pub upper_percentile: f64,
    pub lower: f64,
    pub upper: f64,
    /// The expected count of resamples below the lower percentile was under `MIN_TAIL_RESAMPLES`.
    pub insufficient_tail: bool,
}

/// Runs the paired bootstrap.
pub fn paired_bootstrap(
    items: &[PairedItem],
    seed: u64,
    resamples: usize,
    hypotheses: usize,
) -> Result<Bootstrap> {
    if resamples == 0 || hypotheses == 0 {
        return Err(Error::Stats(
            "resamples and hypotheses must be positive".into(),
        ));
    }
    let families = canonical_families(items)?;
    if families.is_empty() {
        return Err(Error::Stats("no family has an expected-fail item".into()));
    }
    let all: Vec<usize> = (0..families.len()).collect();
    let point = estimand(&families, &all);
    let mut estimates: Vec<f64> = resample_indices(seed, families.len(), resamples)
        .iter()
        .map(|picks| estimand(&families, picks))
        .collect();
    estimates.sort_by(f64::total_cmp);
    let lower_percentile = 0.025 / hypotheses as f64;
    let upper_percentile = 1.0 - lower_percentile;
    Ok(Bootstrap {
        seed,
        resamples,
        hypotheses,
        families: families.len(),
        differing_families: families.iter().filter(|f| f.differs()).count(),
        point,
        lower_percentile,
        upper_percentile,
        lower: quantile_type7(&estimates, lower_percentile),
        upper: quantile_type7(&estimates, upper_percentile),
        insufficient_tail: lower_percentile * (resamples as f64) < MIN_TAIL_RESAMPLES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(family: &str, id: &str, fail: bool, c: bool, x: bool) -> PairedItem {
        PairedItem {
            family: family.into(),
            id: id.into(),
            expected_fail: fail,
            detected_c: c,
            detected_x: x,
        }
    }

    #[test]
    fn stats_canonical_order_sorts_families_and_items_and_drops_families_without_fails() {
        let items = vec![
            item("b", "b-2", true, false, true),
            item("a", "a-1", false, false, false),
            item("b", "b-1", false, false, false),
            item("c", "c-1", true, true, true),
        ];
        let fams = canonical_families(&items).expect("ok");
        assert_eq!(
            fams.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            ["b", "c"]
        );
        assert_eq!(
            fams[0]
                .items
                .iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>(),
            ["b-1", "b-2"]
        );
        assert!(fams[0].differs() && !fams[1].differs());
    }

    #[test]
    fn stats_estimand_is_item_weighted_over_selected_families_with_multiplicity() {
        let fams = canonical_families(&[
            item("a", "a-1", true, false, true),
            item("b", "b-1", true, true, true),
            item("b", "b-2", true, false, true),
            item("b", "b-3", true, false, false),
        ])
        .expect("ok");
        // a: +1 of 1; b: +1 of 3. Both once: 2/4. a twice: (1+1+1)/(1+1+3) = 3/5.
        assert!((estimand(&fams, &[0, 1]) - 0.5).abs() < 1e-12);
        assert!((estimand(&fams, &[0, 0, 1]) - 0.6).abs() < 1e-12);
    }

    #[test]
    fn stats_type7_quantiles_match_hand_values() {
        let x = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(quantile_type7(&x, 0.0), 1.0);
        assert_eq!(quantile_type7(&x, 1.0), 4.0);
        assert_eq!(quantile_type7(&x, 0.5), 2.5);
        assert_eq!(quantile_type7(&x, 0.25), 1.75);
        assert_eq!(quantile_type7(&[7.0], 0.3), 7.0);
        let five = [10.0, 20.0, 30.0, 40.0, 50.0];
        assert!((quantile_type7(&five, 0.1) - 14.0).abs() < 1e-12, "h = 0.4");
    }

    #[test]
    fn stats_resample_indices_are_deterministic_for_a_seed_and_differ_across_seeds() {
        let a = resample_indices(7, 5, 3);
        let b = resample_indices(7, 5, 3);
        assert_eq!(a, b);
        assert_eq!(a.len(), 3);
        assert!(a.iter().all(|r| r.len() == 5 && r.iter().all(|&i| i < 5)));
        assert_ne!(a, resample_indices(8, 5, 3));
    }

    #[test]
    fn stats_bonferroni_percentiles_and_the_insufficient_tail_rule() {
        let items = vec![
            item("a", "a-1", true, false, true),
            item("b", "b-1", true, false, true),
        ];
        let b = paired_bootstrap(&items, 1, 1000, 1).expect("ok");
        assert_eq!((b.lower_percentile, b.upper_percentile), (0.025, 0.975));
        assert!(!b.insufficient_tail, "25 resamples below the percentile");
        let b4 = paired_bootstrap(&items, 1, 1000, 4).expect("ok");
        assert!((b4.lower_percentile - 0.00625).abs() < 1e-12);
        assert!(
            b4.insufficient_tail,
            "6.25 resamples cannot resolve the tail"
        );
        let few = paired_bootstrap(&items, 1, 100, 1).expect("ok");
        assert!(few.insufficient_tail, "2.5 resamples");
    }
}
