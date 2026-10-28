//! Drift (SPEC "Drift"): `dejaview eval canary` reruns a pinned set, uncached, under the
//! configuration as deployed and compares every verdict with the baseline recorded when the
//! configuration was qualified. One deviating canary marks the configuration `suspect`; two
//! consecutive deviations against the same baseline `recommend_disable`; a run in which an item
//! ended in an operational error and nothing else deviated is `inconclusive` (an error is never
//! a verdict). Nothing is ever disabled here: that is the owner's call. `dejaview eval requalify`
//! reruns the full set and records a new baseline on the owner's approval, which resets the
//! canary history. Both run under a spend cap of their own.

use std::collections::BTreeMap;
use std::path::Path;

use dejaview_core::config::Config;
use dejaview_core::image::sha256_hex;
use dejaview_core::ledger::{CapLedger, Ledger};
use dejaview_core::provider::Provider;
use dejaview_core::run::{self, Services};
use dejaview_core::tool::ToolRegistry;
use serde::{Deserialize, Serialize};

use crate::sets::{Seal, Set};
use crate::{Error, Result};

pub const STATE_VERSION: u32 = 1;

/// The provider and tools the configuration resolves to: the binary's real ones, scripted ones
/// in tests.
pub trait Judge {
    fn provider(&self, config: &Config) -> Result<Box<dyn Provider>>;
    fn tools(&self, config: &Config) -> Result<ToolRegistry>;
}

/// The canary state file: the baseline and every canary run against it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryState {
    pub state_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<Baseline>,
    #[serde(default)]
    pub runs: Vec<CanaryRun>,
}

/// What qualification recorded: the configuration's identity and every item's verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    /// Sixteen hex digits over the configuration, the set, the seal and the verdicts.
    pub id: String,
    /// UTC, `YYYY-MM-DDTHH:MM:SSZ`.
    pub created: String,
    /// The owner's words of approval.
    pub approval: String,
    pub set: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seal_sha256: Option<String>,
    /// [`configuration_identity`] of the config the baseline was recorded under.
    pub configuration: String,
    pub verdicts: BTreeMap<String, String>,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryRun {
    pub at: String,
    pub baseline: String,
    pub status: Status,
    pub deviations: Vec<Deviation>,
    /// Items that ended in an operational error: compared with nothing, never a deviation.
    #[serde(default)]
    pub errors: Vec<String>,
    pub verdicts: BTreeMap<String, String>,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deviation {
    pub item: String,
    pub baseline: String,
    pub observed: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Clear,
    Suspect,
    RecommendDisable,
    /// An item errored and nothing deviated: the baseline was neither confirmed nor contradicted.
    Inconclusive,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Status::Clear => "clear",
            Status::Suspect => "suspect",
            Status::RecommendDisable => "recommend_disable",
            Status::Inconclusive => "inconclusive",
        })
    }
}

/// What a canary or requalification runs with: the configuration as deployed, the pinned set,
/// where the decisions go, the judge the configuration resolves to, the harness version and the
/// most the whole run may spend.
pub struct Setup<'a> {
    pub config: &'a Config,
    pub set: &'a Set,
    pub out_dir: &'a Path,
    pub judge: &'a dyn Judge,
    pub harness_version: &'a str,
    pub spend_cap_usd: f64,
}

/// What `requalify` produced: the baseline, whether it was recorded, and how its verdicts differ
/// from the baseline it replaces.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Requalification {
    pub baseline: Baseline,
    pub recorded: bool,
    pub changed: Vec<Deviation>,
}

fn canary_err(what: impl std::fmt::Display) -> Error {
    Error::Canary(what.to_string())
}

impl CanaryState {
    pub fn load(path: &Path) -> Result<CanaryState> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| canary_err(format!("cannot read {}: {e}", path.display())))?;
        let s: CanaryState =
            toml::from_str(&text).map_err(|e| canary_err(format!("{}: {e}", path.display())))?;
        if s.state_version != STATE_VERSION {
            return Err(canary_err(format!(
                "{}: state_version must be {STATE_VERSION}",
                path.display()
            )));
        }
        Ok(s)
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| canary_err(format!("cannot create {}: {e}", parent.display())))?;
        }
        let text = toml::to_string_pretty(self).map_err(canary_err)?;
        crate::write_atomic(path, text.as_bytes())
            .map_err(|e| canary_err(format!("cannot write {}: {e}", path.display())))
    }
}

/// A digest of everything that decides an answer: provider profile, model and controls, rules
/// base, profile and extra files, the enabled tools and their frozen manifests, decision runs,
/// policy, the prompt template, the catalog and the harness version. Budgets, cache and output
/// settings are left out: they change cost and placement, not verdicts.
pub fn configuration_identity(
    config: &Config,
    judge: &dyn Judge,
    harness_version: &str,
) -> Result<String> {
    let registry = judge.tools(config)?;
    let mut manifests: Vec<String> = registry
        .manifests()
        .map(|m| serde_json::to_string(m).unwrap_or_default())
        .collect();
    manifests.sort();
    let rules = config.rules.as_ref().map(|r| {
        serde_json::json!({
            "base": r.base,
            "profile": r.profile,
            "extra": r.extra.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        })
    });
    let v = serde_json::json!({
        "provider": {
            "profile": config.provider.profile,
            "model": config.provider.model,
            "controls": config.provider.controls,
        },
        "rules": rules,
        "tools": config.tools.enabled,
        "manifests": manifests,
        "decision_runs": config.budget.decision_runs,
        "policy": { "on_unsure": config.policy.on_unsure, "on_error": config.policy.on_error },
        "prompt_version": dejaview_core::render::PROMPT_VERSION,
        "prompt_sha256": sha256_hex(dejaview_core::render::TEMPLATE.as_bytes()),
        "catalog_sha256": sha256_hex(dejaview_providers::catalog::SHIPPED.as_bytes()),
        "harness_version": harness_version,
    });
    Ok(sha256_hex(
        serde_json::to_string(&v).map_err(canary_err)?.as_bytes(),
    ))
}

/// Runs every item of the set under the configuration, never reading the cache and never past
/// `spend_cap_usd` in total, and returns each item's verdict (`pass`, `fail`, `unsure` or
/// `error`) and the total charged cost.
pub fn judge_set(setup: &Setup<'_>) -> Result<(BTreeMap<String, String>, f64)> {
    let Setup {
        config,
        set,
        out_dir,
        judge,
        harness_version,
        spend_cap_usd,
    } = *setup;
    if spend_cap_usd.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return Err(canary_err("the spend cap must be positive"));
    }
    let ledger = CapLedger::new(spend_cap_usd);
    let base = &set.manifest.set.rules.base;
    let mut config = config.clone();
    match &config.rules {
        None => {
            config.rules = Some(
                serde_json::from_value(serde_json::json!({ "base": base })).map_err(canary_err)?,
            );
        }
        Some(r) if r.base != *base => {
            return Err(canary_err(format!(
                "the configuration judges {:?}; set {} is a {base:?} set",
                r.base, set.manifest.set.id
            )));
        }
        Some(_) => {}
    }
    std::fs::create_dir_all(out_dir)
        .map_err(|e| canary_err(format!("cannot create {}: {e}", out_dir.display())))?;
    let mut verdicts = BTreeMap::new();
    let mut cost = 0.0;
    for (item, job) in &set.jobs {
        let provider = judge.provider(&config)?;
        let tools = judge.tools(&config)?;
        if let Some(pricing) = provider.pricing() {
            let envelope = run::envelope(&config, job, &set.dir, &tools, pricing)
                .map_err(|e| canary_err(format!("{item}: {e}")))?;
            let remaining = Ledger::cap(&ledger) - Ledger::spent(&ledger);
            if envelope.usd > remaining + 1e-9 {
                return Err(canary_err(format!(
                    "{item}: its envelope ${:.4} exceeds the ${remaining:.4} left of the ${spend_cap_usd:.2} spend cap; nothing more was sent",
                    envelope.usd
                )));
            }
        }
        let mut services =
            Services::new(provider.as_ref(), &tools).with_shared_ledger(Some(&ledger));
        services.evaluation = true;
        let outcome = crate::runner::block_on(run::check(
            &config,
            job,
            &set.dir,
            &services,
            out_dir,
            harness_version,
        ))?
        .map_err(|e| canary_err(format!("{item}: {e}")))?;
        let verdict = outcome
            .decision
            .verdict
            .map(|v| format!("{v:?}").to_ascii_lowercase())
            .unwrap_or_else(|| "error".to_string());
        let report: serde_json::Value =
            std::fs::read_to_string(outcome.dir.join(dejaview_core::report::REPORT_FILE))
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or(serde_json::Value::Null);
        cost += report["cost"]["spent_usd"].as_f64().unwrap_or(0.0);
        verdicts.insert(item.clone(), verdict);
    }
    Ok((verdicts, cost))
}

/// Deviations between a baseline and an observation, leaving errored observations aside.
fn deviations(
    baseline: &BTreeMap<String, String>,
    observed: &BTreeMap<String, String>,
) -> Vec<Deviation> {
    let mut out = Vec::new();
    for (item, want) in baseline {
        if observed.get(item).is_some_and(|v| v == "error") {
            continue;
        }
        let got = observed
            .get(item)
            .cloned()
            .unwrap_or_else(|| "missing".into());
        if got != *want {
            out.push(Deviation {
                item: item.clone(),
                baseline: want.clone(),
                observed: got,
            });
        }
    }
    for (item, got) in observed {
        if !baseline.contains_key(item) {
            out.push(Deviation {
                item: item.clone(),
                baseline: "missing".into(),
                observed: got.clone(),
            });
        }
    }
    out
}

fn seal_of(set: &Set) -> Option<String> {
    Seal::load(&set.dir).ok().map(|s| s.seal.sha256)
}

/// One canary run against the stored baseline; appends to the state file.
pub fn canary(state_path: &Path, setup: &Setup<'_>) -> Result<CanaryRun> {
    let (config, set, judge, harness_version) =
        (setup.config, setup.set, setup.judge, setup.harness_version);
    let mut state = CanaryState::load(state_path)
        .map_err(|e| canary_err(format!("{e}; run `eval requalify` to record a baseline")))?;
    let baseline = state.baseline.as_ref().ok_or_else(|| {
        canary_err(format!(
            "{} holds no baseline; run `eval requalify` first",
            state_path.display()
        ))
    })?;
    let identity = configuration_identity(config, judge, harness_version)?;
    if baseline.configuration != identity {
        return Err(canary_err(
            "the configuration differs from the one the baseline was recorded under; a canary compares like with like, so requalify first",
        ));
    }
    if baseline.set != set.manifest.set.id {
        return Err(canary_err(format!(
            "the baseline was recorded on {}, not {}",
            baseline.set, set.manifest.set.id
        )));
    }
    if baseline.seal_sha256.is_some() && seal_of(set) != baseline.seal_sha256 {
        return Err(canary_err(format!(
            "set {} is not sealed as it was when the baseline was recorded",
            set.manifest.set.id
        )));
    }
    let (verdicts, cost_usd) = judge_set(setup)?;
    let errors: Vec<String> = verdicts
        .iter()
        .filter(|(_, v)| *v == "error")
        .map(|(item, _)| item.clone())
        .collect();
    let deviations = deviations(&baseline.verdicts, &verdicts);
    let previous_deviated = state.runs.last().is_some_and(|r| {
        r.baseline == baseline.id && matches!(r.status, Status::Suspect | Status::RecommendDisable)
    });
    let status = if !deviations.is_empty() && previous_deviated {
        Status::RecommendDisable
    } else if !deviations.is_empty() {
        Status::Suspect
    } else if !errors.is_empty() {
        Status::Inconclusive
    } else {
        Status::Clear
    };
    let run = CanaryRun {
        at: crate::sets::utc_now(),
        baseline: baseline.id.clone(),
        status,
        deviations,
        errors,
        verdicts,
        cost_usd,
    };
    state.runs.push(run.clone());
    state.write(state_path)?;
    Ok(run)
}

/// Reruns the full set and, with the owner's approval, records the result as the new baseline
/// and clears the canary history. Without approval nothing is written: the would-be baseline and
/// how it differs from the current one are returned for the owner to look at.
pub fn requalify(
    state_path: &Path,
    setup: &Setup<'_>,
    approval: Option<&str>,
) -> Result<Requalification> {
    let (config, set, judge, harness_version) =
        (setup.config, setup.set, setup.judge, setup.harness_version);
    let (verdicts, cost_usd) = judge_set(setup)?;
    if verdicts.values().any(|v| v == "error") {
        return Err(canary_err(
            "an item ended in an operational error; a baseline records verdicts only, so run again when the provider is well",
        ));
    }
    let configuration = configuration_identity(config, judge, harness_version)?;
    let seal_sha256 = seal_of(set);
    let mut text = format!(
        "{configuration}\n{}\n{}\n",
        set.manifest.set.id,
        seal_sha256.as_deref().unwrap_or("")
    );
    for (item, v) in &verdicts {
        text.push_str(&format!("{item}={v}\n"));
    }
    let baseline = Baseline {
        id: sha256_hex(text.as_bytes())[..16].to_string(),
        created: crate::sets::utc_now(),
        approval: approval.unwrap_or("").to_string(),
        set: set.manifest.set.id.clone(),
        seal_sha256,
        configuration,
        verdicts,
        cost_usd,
    };
    let mut state = if state_path.exists() {
        CanaryState::load(state_path)?
    } else {
        CanaryState {
            state_version: STATE_VERSION,
            baseline: None,
            runs: Vec::new(),
        }
    };
    let changed = state
        .baseline
        .as_ref()
        .map(|b| deviations(&b.verdicts, &baseline.verdicts))
        .unwrap_or_default();
    let recorded = approval.is_some_and(|a| !a.trim().is_empty());
    if recorded {
        state.baseline = Some(baseline.clone());
        state.runs.clear();
        state.write(state_path)?;
    }
    Ok(Requalification {
        baseline,
        recorded,
        changed,
    })
}
