//! The admission gate (SPEC "Gates"): per-arm metrics from a paired result table, the gate table
//! with its defaults, and the outcome: `admitted` when every gate holds; `rejected` when detected
//! failures are not higher or missed failures or errors are higher; `inconclusive` otherwise,
//! including too few differing families or a tail the resamples cannot resolve.

use dejaview_core::config::PolicyOutcome;
use serde::{Deserialize, Serialize};

use crate::bootstrap::Bootstrap;

/// A computed verdict for one item under one arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Computed {
    Pass,
    Fail,
    Unsure,
    Error,
}

/// What the consumer's frozen policy makes of unsure and error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub on_unsure: PolicyOutcome,
    pub on_error: PolicyOutcome,
}

impl Policy {
    /// Whether this computed verdict lets an item through as a pass.
    pub fn passes(&self, computed: Computed) -> bool {
        match computed {
            Computed::Pass => true,
            Computed::Fail => false,
            Computed::Unsure => self.on_unsure == PolicyOutcome::Pass,
            Computed::Error => self.on_error == PolicyOutcome::Pass,
        }
    }
}

/// One labelled item with its computed verdict under one arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scored {
    pub expected_fail: bool,
    pub computed: Computed,
}

/// Per-arm counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ArmMetrics {
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
    pub cost_usd: f64,
}

impl ArmMetrics {
    pub fn from_items(items: &[Scored], policy: &Policy, cost_usd: f64) -> ArmMetrics {
        let mut m = ArmMetrics {
            items: items.len(),
            cost_usd,
            ..ArmMetrics::default()
        };
        for s in items {
            if s.expected_fail {
                m.expected_fail += 1;
                if s.computed == Computed::Fail {
                    m.detected += 1;
                } else if policy.passes(s.computed) {
                    m.missed += 1;
                }
            } else {
                m.expected_pass += 1;
                if s.computed == Computed::Fail {
                    m.false_rejects += 1;
                }
            }
            match s.computed {
                Computed::Unsure => m.unsure += 1,
                Computed::Error => m.errors += 1,
                _ => {}
            }
        }
        m
    }

    pub fn error_rate(&self) -> f64 {
        rate(self.errors, self.items)
    }

    pub fn unsure_rate(&self) -> f64 {
        rate(self.unsure, self.items)
    }
}

fn rate(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        count as f64 / total as f64
    }
}

/// The gate table's thresholds (SPEC "Gates", defaults).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thresholds {
    pub max_error_rate: f64,
    pub min_detected_gain: usize,
    pub max_false_reject_gain: usize,
    pub max_unsure_rate: f64,
    pub max_cost_ratio: f64,
    pub min_families: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            max_error_rate: 0.05,
            min_detected_gain: 3,
            max_false_reject_gain: 1,
            max_unsure_rate: 0.25,
            max_cost_ratio: 2.0,
            min_families: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Admitted,
    Rejected,
    Inconclusive,
}

/// The outcome with every gate's finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateVerdict {
    pub outcome: Outcome,
    /// Gates that did not hold, in table order.
    pub failed: Vec<String>,
}

/// Applies the gate table to arm C (control) and arm X (the tool under test).
pub fn decide(
    c: &ArmMetrics,
    x: &ArmMetrics,
    bootstrap: &Bootstrap,
    t: &Thresholds,
) -> GateVerdict {
    let mut failed = Vec::new();
    let mut rejected = false;
    if x.missed > c.missed {
        failed.push("missed failures: higher than C".into());
        rejected = true;
    }
    if x.errors > c.errors {
        failed.push("error rate: higher than C".into());
        rejected = true;
    }
    if x.error_rate() > t.max_error_rate {
        failed.push(format!(
            "error rate: above {:.0}%",
            t.max_error_rate * 100.0
        ));
    }
    if x.detected <= c.detected {
        failed.push("detected failures: not higher than C".into());
        rejected = true;
    } else if x.detected - c.detected < t.min_detected_gain {
        failed.push(format!(
            "detected failures: fewer than {} more items",
            t.min_detected_gain
        ));
    }
    if bootstrap.lower <= 0.0 {
        failed.push("detected failures: paired lower bound not above 0".into());
    }
    if x.false_rejects > c.false_rejects + t.max_false_reject_gain {
        failed.push(format!(
            "false rejects: more than {} more items",
            t.max_false_reject_gain
        ));
    }
    if x.unsure_rate() > t.max_unsure_rate {
        failed.push(format!(
            "unsure rate: above {:.0}%",
            t.max_unsure_rate * 100.0
        ));
    }
    if c.cost_usd > 0.0 && x.cost_usd > t.max_cost_ratio * c.cost_usd {
        failed.push(format!("cost: more than {}x C", t.max_cost_ratio));
    }
    if bootstrap.differing_families < t.min_families {
        failed.push(format!(
            "independent families: {} differing, fewer than {}",
            bootstrap.differing_families, t.min_families
        ));
    }
    if bootstrap.insufficient_tail {
        failed.push("bootstrap: too few resamples to resolve the adjusted tail".into());
    }
    let outcome = if rejected {
        Outcome::Rejected
    } else if failed.is_empty() {
        Outcome::Admitted
    } else {
        Outcome::Inconclusive
    };
    GateVerdict { outcome, failed }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> Policy {
        Policy {
            on_unsure: PolicyOutcome::Review,
            on_error: PolicyOutcome::Review,
        }
    }

    #[test]
    fn stats_arm_metrics_count_missed_only_when_the_policy_lets_it_through() {
        let items = [
            Scored {
                expected_fail: true,
                computed: Computed::Fail,
            },
            Scored {
                expected_fail: true,
                computed: Computed::Pass,
            },
            Scored {
                expected_fail: true,
                computed: Computed::Unsure,
            },
            Scored {
                expected_fail: false,
                computed: Computed::Fail,
            },
            Scored {
                expected_fail: false,
                computed: Computed::Error,
            },
        ];
        let m = ArmMetrics::from_items(&items, &policy(), 1.0);
        assert_eq!(
            (m.detected, m.missed, m.false_rejects, m.unsure, m.errors),
            (1, 1, 1, 1, 1)
        );
        let lenient = Policy {
            on_unsure: PolicyOutcome::Pass,
            on_error: PolicyOutcome::Pass,
        };
        assert_eq!(
            ArmMetrics::from_items(&items, &lenient, 1.0).missed,
            2,
            "unsure let through counts"
        );
        assert!((m.error_rate() - 0.2).abs() < 1e-12);
    }

    fn bootstrap(lower: f64, differing: usize, insufficient: bool) -> Bootstrap {
        Bootstrap {
            seed: 1,
            resamples: 1000,
            hypotheses: 1,
            families: 6,
            differing_families: differing,
            point: 0.5,
            lower_percentile: 0.025,
            upper_percentile: 0.975,
            lower,
            upper: 0.9,
            insufficient_tail: insufficient,
        }
    }

    fn arm(
        detected: usize,
        missed: usize,
        errors: usize,
        false_rejects: usize,
        unsure: usize,
        cost: f64,
    ) -> ArmMetrics {
        ArmMetrics {
            items: 20,
            expected_fail: 10,
            expected_pass: 10,
            detected,
            missed,
            false_rejects,
            unsure,
            errors,
            cost_usd: cost,
        }
    }

    #[test]
    fn stats_gate_outcomes() {
        let c = arm(2, 6, 0, 0, 1, 1.0);
        let good = arm(7, 1, 0, 1, 0, 1.5);
        assert_eq!(
            decide(&c, &good, &bootstrap(0.1, 5, false), &Thresholds::default()).outcome,
            Outcome::Admitted
        );
        // Not higher detection, or more misses or errors: rejected.
        assert_eq!(
            decide(
                &c,
                &arm(2, 1, 0, 0, 0, 1.0),
                &bootstrap(0.1, 5, false),
                &Thresholds::default()
            )
            .outcome,
            Outcome::Rejected
        );
        assert_eq!(
            decide(
                &c,
                &arm(7, 7, 0, 0, 0, 1.0),
                &bootstrap(0.1, 5, false),
                &Thresholds::default()
            )
            .outcome,
            Outcome::Rejected
        );
        assert_eq!(
            decide(
                &c,
                &arm(7, 1, 1, 0, 0, 1.0),
                &bootstrap(0.1, 5, false),
                &Thresholds::default()
            )
            .outcome,
            Outcome::Rejected
        );
        // Otherwise anything short is inconclusive.
        assert_eq!(
            decide(&c, &good, &bootstrap(0.1, 4, false), &Thresholds::default()).outcome,
            Outcome::Inconclusive
        );
        assert_eq!(
            decide(&c, &good, &bootstrap(0.1, 5, true), &Thresholds::default()).outcome,
            Outcome::Inconclusive
        );
        assert_eq!(
            decide(&c, &good, &bootstrap(0.0, 5, false), &Thresholds::default()).outcome,
            Outcome::Inconclusive
        );
        assert_eq!(
            decide(
                &c,
                &arm(4, 1, 0, 0, 0, 1.0),
                &bootstrap(0.1, 5, false),
                &Thresholds::default()
            )
            .outcome,
            Outcome::Inconclusive,
            "gain of 2"
        );
        assert_eq!(
            decide(
                &c,
                &arm(7, 1, 0, 2, 0, 1.0),
                &bootstrap(0.1, 5, false),
                &Thresholds::default()
            )
            .outcome,
            Outcome::Inconclusive,
            "two more false rejects"
        );
        assert_eq!(
            decide(
                &c,
                &arm(7, 1, 0, 0, 6, 1.0),
                &bootstrap(0.1, 5, false),
                &Thresholds::default()
            )
            .outcome,
            Outcome::Inconclusive,
            "30% unsure"
        );
        assert_eq!(
            decide(
                &c,
                &arm(7, 1, 0, 0, 0, 2.5),
                &bootstrap(0.1, 5, false),
                &Thresholds::default()
            )
            .outcome,
            Outcome::Inconclusive,
            "2.5x cost"
        );
    }
}
