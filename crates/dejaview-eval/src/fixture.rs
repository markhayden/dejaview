//! A synthetic paired result table for the statistics contract (`fixtures/stats/*.toml`): the
//! items with their expected label and computed verdict per arm, the frozen policy and
//! thresholds, the seed, and a `golden` section the implementation must reproduce exactly.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::bootstrap::{self, Bootstrap, PairedItem};
use crate::gate_rules::{
    ArmMetrics, Computed, GateVerdict, Outcome, Policy, Scored, Thresholds, decide,
};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatsFixture {
    pub seed: u64,
    pub resamples: usize,
    pub hypotheses: usize,
    pub policy: Policy,
    #[serde(default)]
    pub thresholds: Thresholds,
    pub cost_c_usd: f64,
    pub cost_x_usd: f64,
    pub items: Vec<FixtureItem>,
    /// Written once by the implementation, then reproduced exactly from the seed.
    #[serde(default)]
    pub golden: Option<Golden>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureItem {
    pub family: String,
    pub id: String,
    pub expected: Expected,
    pub c: Computed,
    pub x: Computed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expected {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Golden {
    /// The first resamples' family indices.
    pub first_resamples: Vec<Vec<usize>>,
    pub point: f64,
    pub lower: f64,
    pub upper: f64,
    pub differing_families: usize,
    pub outcome: Outcome,
}

/// Everything the fixture evaluates to.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Evaluation {
    pub c: ArmMetrics,
    pub x: ArmMetrics,
    pub bootstrap: Bootstrap,
    pub verdict: GateVerdict,
    pub first_resamples: Vec<Vec<usize>>,
}

/// How many resamples the golden section records.
pub const GOLDEN_RESAMPLES: usize = 3;

impl StatsFixture {
    pub fn load(path: &Path) -> Result<StatsFixture> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Stats(format!("cannot read {}: {e}", path.display())))?;
        toml::from_str(&text).map_err(|e| Error::Stats(format!("{}: {e}", path.display())))
    }

    pub fn paired_items(&self) -> Vec<PairedItem> {
        self.items
            .iter()
            .map(|i| PairedItem {
                family: i.family.clone(),
                id: i.id.clone(),
                expected_fail: i.expected == Expected::Fail,
                detected_c: i.c == Computed::Fail,
                detected_x: i.x == Computed::Fail,
            })
            .collect()
    }

    fn scored(&self, arm: impl Fn(&FixtureItem) -> Computed) -> Vec<Scored> {
        self.items
            .iter()
            .map(|i| Scored {
                expected_fail: i.expected == Expected::Fail,
                computed: arm(i),
            })
            .collect()
    }

    pub fn evaluate(&self) -> Result<Evaluation> {
        let c = ArmMetrics::from_items(&self.scored(|i| i.c), &self.policy, self.cost_c_usd);
        let x = ArmMetrics::from_items(&self.scored(|i| i.x), &self.policy, self.cost_x_usd);
        let items = self.paired_items();
        let boot = bootstrap::paired_bootstrap(&items, self.seed, self.resamples, self.hypotheses)?;
        let verdict = decide(&c, &x, &boot, &self.thresholds);
        let first_resamples =
            bootstrap::resample_indices(self.seed, boot.families, GOLDEN_RESAMPLES);
        Ok(Evaluation {
            c,
            x,
            bootstrap: boot,
            verdict,
            first_resamples,
        })
    }

    /// The golden section this evaluation implies.
    pub fn golden_of(e: &Evaluation) -> Golden {
        Golden {
            first_resamples: e.first_resamples.clone(),
            point: e.bootstrap.point,
            lower: e.bootstrap.lower,
            upper: e.bootstrap.upper,
            differing_families: e.bootstrap.differing_families,
            outcome: e.verdict.outcome,
        }
    }
}
