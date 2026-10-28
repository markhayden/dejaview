//! `dejaview eval estimate` (SPEC "Runner and arms"): every decision's envelope as the ledger
//! would reserve it, their sum, a cost range, and a warning when the sum exceeds the cap. It
//! never refuses: admission is per decision, and settled costs are usually far below the
//! envelopes, so a matrix whose envelopes sum past the cap can still complete. It also names
//! the decisions whose envelope exceeds the config's per-decision cap, which the run would refuse.

use dejaview_core::run;
use serde::{Deserialize, Serialize};

use crate::arms::tools_for;
use crate::runner::{Factories, Plan, cell_config};
use crate::sets::Set;
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionEstimate {
    pub id: String,
    /// The envelope the ledger reserves for the decision.
    pub envelope_usd: f64,
    /// The bound of the first request alone: what a decision that submits at once costs at most.
    pub first_request_usd: f64,
    /// The envelope exceeds the config's per-decision cap (`budget.spend_cap_usd`): the decision
    /// will be refused with `error: budget` whatever the matrix cap allows.
    pub over_decision_cap: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Estimate {
    pub matrix: String,
    pub decisions: Vec<DecisionEstimate>,
    pub envelope_sum_usd: f64,
    /// The sum of first-request bounds: a floor for a matrix where every decision submits at
    /// once.
    pub floor_usd: f64,
    pub cap_usd: f64,
    /// The envelope sum exceeds the cap: completion is not guaranteed.
    pub over_cap: bool,
    /// Decisions whose envelope exceeds the per-decision cap, which the run would refuse.
    pub over_decision_cap: Vec<String>,
}

pub fn estimate(plan: &Plan, factories: &dyn Factories) -> Result<Estimate> {
    let m = &plan.matrix.matrix;
    let admissions = plan.admissions.clone().unwrap_or_default();
    let cells = plan.matrix.cells();
    let mut decisions = Vec::new();
    for id in &m.sets {
        let set = Set::load(&plan.evalsets_root.join(id))?;
        let use_case = set.manifest.set.use_case.clone();
        for (item_id, job) in &set.jobs {
            for cell in &cells {
                let selection = tools_for(&cell.arm, &admissions, &use_case)?;
                let config = cell_config(plan, cell, &set, &selection)?;
                let provider = factories.provider(cell, &config)?;
                let tools = factories.tools(&selection, &config)?;
                let pricing = provider.pricing().ok_or_else(|| {
                    Error::Runner(format!(
                        "cell {}: the provider cannot bound request costs",
                        cell.id()
                    ))
                })?;
                let envelope =
                    run::envelope(&config, job, &set.dir, &tools, pricing).map_err(|e| {
                        Error::Runner(format!(
                            "{}/{item_id}/{}: {e}",
                            set.manifest.set.id,
                            cell.id()
                        ))
                    })?;
                let first = envelope
                    .requests
                    .first()
                    .map(|r| r.bound_usd)
                    .unwrap_or(0.0);
                for repetition in 1..=m.repetitions {
                    decisions.push(DecisionEstimate {
                        id: format!(
                            "{}/{item_id}/{}/rep-{repetition}",
                            set.manifest.set.id,
                            cell.id()
                        ),
                        envelope_usd: envelope.usd,
                        first_request_usd: first * f64::from(m.decision_runs),
                        over_decision_cap: envelope.usd > config.budget.spend_cap_usd,
                    });
                }
            }
        }
    }
    let envelope_sum_usd: f64 = decisions.iter().map(|d| d.envelope_usd).sum();
    let floor_usd: f64 = decisions.iter().map(|d| d.first_request_usd).sum();
    let over_decision_cap = decisions
        .iter()
        .filter(|d| d.over_decision_cap)
        .map(|d| d.id.clone())
        .collect();
    Ok(Estimate {
        matrix: m.name.clone(),
        decisions,
        envelope_sum_usd,
        floor_usd,
        cap_usd: m.spend_cap_usd,
        over_cap: envelope_sum_usd > m.spend_cap_usd,
        over_decision_cap,
    })
}
