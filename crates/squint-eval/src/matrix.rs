//! `matrix.toml` (SPEC "Runner and arms"): the sets, the models, the arms, the decision runs and
//! repetitions per cell, and the evaluation's spend cap. A cell is one model under one arm; every
//! item of every set runs through every cell `repetitions` times. The matrix identity is a hash
//! of its content, used by the descriptor and the gate.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use squint_core::image::sha256_hex;

use crate::gate_rules::Thresholds;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Matrix {
    pub matrix: MatrixSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixSpec {
    pub name: String,
    /// The base config every cell starts from (budget, policy, privacy, rules); the provider and
    /// tools sections are set per cell.
    pub config: PathBuf,
    /// Set ids under the evalsets root.
    pub sets: Vec<String>,
    pub models: Vec<ModelSpec>,
    pub arms: Vec<Arm>,
    pub decision_runs: u32,
    pub repetitions: u32,
    pub spend_cap_usd: f64,
    /// The frozen gate configuration (SPEC "Gates"); required by `eval gate`, part of the
    /// identity like everything else here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateSpec>,
}

/// What a gate freezes before unsealing besides the cells: the bootstrap seed and resample
/// count, the hypothesis count for the Bonferroni adjustment, and the thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateSpec {
    pub seed: u64,
    #[serde(default = "default_resamples")]
    pub resamples: usize,
    /// Defaults to models × `D_<tool>` arms, the hypotheses the matrix declares on its sets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hypotheses: Option<usize>,
    #[serde(default)]
    pub thresholds: Thresholds,
}

fn default_resamples() -> usize {
    10_000
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSpec {
    pub profile: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub controls: BTreeMap<String, toml::Value>,
    /// The `[privacy] account` for this model's cells, when the base config's does not bind to
    /// its profile (a matrix across vendors). Write it as the config would, with the account id
    /// from the environment (`google:${SQUINT_GOOGLE_ACCOUNT}`): the reference is what the
    /// matrix carries and hashes, the value is expanded at run time and redacted like any secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
}

/// An arm (SPEC "Runner and arms"): no tools, the visual-access control, the control plus one
/// tool under test, or the control plus every admitted tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arm {
    A,
    C,
    DX(String),
    D,
}

impl FromStr for Arm {
    type Err = Error;

    fn from_str(s: &str) -> Result<Arm> {
        match s {
            "A" => Ok(Arm::A),
            "C" => Ok(Arm::C),
            "D" => Ok(Arm::D),
            other => match other.strip_prefix("D_") {
                Some(tool) if !tool.is_empty() => Ok(Arm::DX(tool.to_string())),
                _ => Err(Error::Matrix(format!(
                    "arm {s:?} is not A, C, D_<tool> or D"
                ))),
            },
        }
    }
}

impl std::fmt::Display for Arm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arm::A => f.write_str("A"),
            Arm::C => f.write_str("C"),
            Arm::DX(tool) => write!(f, "D_{tool}"),
            Arm::D => f.write_str("D"),
        }
    }
}

impl Serialize for Arm {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Arm {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Arm, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// One model under one arm.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub model: ModelSpec,
    pub arm: Arm,
}

impl Cell {
    /// `<profile>:<model>/<arm>`, the cell's name in results and run directories.
    pub fn id(&self) -> String {
        format!("{}:{}/{}", self.model.profile, self.model.model, self.arm)
    }
}

impl Matrix {
    pub fn parse(text: &str) -> Result<Matrix> {
        let m: Matrix = toml::from_str(text).map_err(|e| Error::Matrix(e.to_string()))?;
        m.validate()?;
        Ok(m)
    }

    pub fn load(path: &Path) -> Result<Matrix> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Matrix(format!("cannot read {}: {e}", path.display())))?;
        Matrix::parse(&text).map_err(|e| Error::Matrix(format!("{}: {e}", path.display())))
    }

    fn validate(&self) -> Result<()> {
        let m = &self.matrix;
        if m.name.is_empty() {
            return Err(Error::Matrix("name must be set".into()));
        }
        if m.sets.is_empty() {
            return Err(Error::Matrix("sets must name at least one set".into()));
        }
        if m.models.is_empty() {
            return Err(Error::Matrix("models must name at least one model".into()));
        }
        if m.arms.is_empty() {
            return Err(Error::Matrix("arms must name at least one arm".into()));
        }
        if !(1..=2).contains(&m.decision_runs) {
            return Err(Error::Matrix("decision_runs must be 1 or 2".into()));
        }
        if m.repetitions == 0 {
            return Err(Error::Matrix("repetitions must be at least 1".into()));
        }
        if m.spend_cap_usd.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(Error::Matrix("spend_cap_usd must be positive".into()));
        }
        for model in &m.models {
            for (key, value) in &model.controls {
                if let toml::Value::Float(f) = value
                    && !f.is_finite()
                {
                    return Err(Error::Matrix(format!(
                        "model {}:{}: control {key:?} must be a finite number",
                        model.profile, model.model
                    )));
                }
            }
        }
        if let Some(g) = &m.gate {
            if g.resamples == 0 {
                return Err(Error::Matrix("gate.resamples must be positive".into()));
            }
            if g.hypotheses == Some(0) {
                return Err(Error::Matrix("gate.hypotheses must be positive".into()));
            }
        }
        Ok(())
    }

    /// The hypotheses the matrix declares: `gate.hypotheses`, or models × `D_<tool>` arms (at
    /// least 1).
    pub fn hypotheses(&self) -> usize {
        let m = &self.matrix;
        m.gate
            .as_ref()
            .and_then(|g| g.hypotheses)
            .unwrap_or_else(|| {
                let tools = m.arms.iter().filter(|a| matches!(a, Arm::DX(_))).count();
                (m.models.len() * tools).max(1)
            })
    }

    /// Every model under every arm, models outer, arms inner.
    pub fn cells(&self) -> Vec<Cell> {
        let mut out = Vec::new();
        for model in &self.matrix.models {
            for arm in &self.matrix.arms {
                out.push(Cell {
                    model: model.clone(),
                    arm: arm.clone(),
                });
            }
        }
        out
    }

    /// Decisions the matrix makes over `items` items: items × cells × repetitions.
    pub fn decisions(&self, items: usize) -> usize {
        items * self.cells().len() * self.matrix.repetitions as usize
    }

    /// The frozen identity: a hash over the matrix content in canonical form, independent of
    /// the file's formatting.
    pub fn identity(&self) -> String {
        let canonical = serde_json::to_string(self).unwrap_or_default();
        sha256_hex(canonical.as_bytes())
    }
}
