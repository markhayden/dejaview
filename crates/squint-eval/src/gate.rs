//! The gate lifecycle (SPEC "Gates", stages): `squint eval gate` scores one hypothesis (one
//! tool, one model, one sealed set) from a finished evaluation directory, under the frozen
//! configuration the matrix carries. It verifies the seal against the supplied labels and the
//! results against the descriptor and the frozen matrix identity, refuses to score a set twice
//! without a new ADR reference, writes a durable receipt, writes the label publication and split
//! change into the set directory for the owner to review and commit (it never commits), updates
//! `admissions.toml`, and resumes an interrupted scoring from its receipt without rescoring.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use squint_core::image::sha256_hex;

use crate::admissions::{Admission, Admissions, Stage};
use crate::bootstrap::{self, Bootstrap, PairedItem};
use crate::descriptor::Descriptor;
use crate::gate_rules::{ArmMetrics, Computed, Outcome, Policy, Scored, decide};
use crate::matrix::{Arm, Matrix, ModelSpec};
use crate::runner::{SUMMARY_FILE, Summary};
use crate::sets::{Expected, Labels, MANIFEST_FILE, Seal, Set, Split, verify_seal};
use crate::summary;
use crate::{Error, Result};

/// Receipts live under the evalsets root, one file per receipt id.
pub const RECEIPTS_DIR: &str = "receipts";
pub const RECEIPT_VERSION: u32 = 1;
pub const MIN_INITIAL_ITEMS: usize = 20;
pub const MIN_CONFIRMATION_ITEMS: usize = 50;
/// The label file a publication writes beside the manifest.
pub const PUBLISHED_LABELS: &str = "labels.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateStage {
    Initial,
    Confirmation,
}

impl FromStr for GateStage {
    type Err = Error;

    fn from_str(s: &str) -> Result<GateStage> {
        match s {
            "initial" => Ok(GateStage::Initial),
            "confirmation" => Ok(GateStage::Confirmation),
            other => Err(Error::Gate(format!(
                "stage {other:?} is not initial or confirmation"
            ))),
        }
    }
}

impl std::fmt::Display for GateStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GateStage::Initial => "initial",
            GateStage::Confirmation => "confirmation",
        })
    }
}

/// What a gate is asked to score.
pub struct Request<'a> {
    /// The evaluation directory a finished `eval run` wrote.
    pub eval_dir: &'a Path,
    /// The matrix the run was made from; its identity must match the descriptor's.
    pub matrix: &'a Matrix,
    pub evalsets_root: &'a Path,
    /// The set id (`<use-case>/<set>`).
    pub set: &'a str,
    pub tool: &'a str,
    /// `<profile>:<model>`; may be omitted when the matrix has one model.
    pub model: Option<&'a str>,
    pub stage: GateStage,
    /// The label file for a blind set (held by the owner outside the repository).
    pub labels: Option<&'a Path>,
    /// The initial stage's receipt id, required by the confirmation stage.
    pub initial_receipt: Option<&'a str>,
    /// An ADR reference permitting a set to be scored again.
    pub adr: Option<&'a str>,
    /// `admissions.toml`, created when missing.
    pub admissions: &'a Path,
}

/// `receipts/<id>.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub receipt: ReceiptBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptBody {
    pub receipt_version: u32,
    /// The first sixteen hex digits of a SHA-256 over stage, tool, set, matrix identity, results
    /// hash and seal hash: the same scoring always has the same id.
    pub id: String,
    /// UTC, `YYYY-MM-DDTHH:MM:SSZ`.
    pub created: String,
    pub stage: GateStage,
    pub tool: String,
    pub use_case: String,
    pub set: String,
    /// `<profile>:<model>`.
    pub model: String,
    pub items: usize,
    pub matrix_identity: String,
    pub descriptor_sha256: String,
    pub seal_sha256: String,
    pub labels_sha256: String,
    /// Over `summary.json`.
    pub results_sha256: String,
    /// Over the judging configuration: harness, config, prompt, catalog, tools, model, decision
    /// runs, policy and thresholds. The confirmation stage requires the initial stage's.
    pub judging_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_receipt: Option<String>,
    pub outcome: Outcome,
    pub failed: Vec<String>,
    pub c: ArmMetrics,
    pub x: ArmMetrics,
    pub bootstrap: Bootstrap,
}

/// What a gate did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GateReport {
    pub receipt: Receipt,
    /// The receipt already existed: nothing was rescored.
    pub resumed: bool,
    /// Files written or rewritten for the owner to review and commit.
    pub written: Vec<PathBuf>,
    /// The admission record as it stands after this gate, if any.
    pub admission: Option<Stage>,
}

fn gate_err(what: impl std::fmt::Display) -> Error {
    Error::Gate(what.to_string())
}

fn io(path: &Path, e: std::io::Error) -> Error {
    Error::Gate(format!("{}: {e}", path.display()))
}

pub fn receipt_path(evalsets_root: &Path, id: &str) -> PathBuf {
    evalsets_root.join(RECEIPTS_DIR).join(format!("{id}.toml"))
}

impl Receipt {
    pub fn load(path: &Path) -> Result<Receipt> {
        let text = std::fs::read_to_string(path).map_err(|e| io(path, e))?;
        let r: Receipt =
            toml::from_str(&text).map_err(|e| Error::Gate(format!("{}: {e}", path.display())))?;
        if r.receipt.receipt_version != RECEIPT_VERSION {
            return Err(Error::Gate(format!(
                "{}: receipt_version must be {RECEIPT_VERSION}",
                path.display()
            )));
        }
        Ok(r)
    }

    /// Every receipt under the evalsets root, by id.
    pub fn load_all(evalsets_root: &Path) -> Result<BTreeMap<String, Receipt>> {
        let dir = evalsets_root.join(RECEIPTS_DIR);
        let mut out = BTreeMap::new();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Ok(out);
        };
        for entry in entries {
            let path = entry.map_err(|e| io(&dir, e))?.path();
            if path.extension().is_some_and(|e| e == "toml") {
                let r = Receipt::load(&path)?;
                out.insert(r.receipt.id.clone(), r);
            }
        }
        Ok(out)
    }
}

/// The judging configuration digest (SPEC "Gates": the identical frozen judging configuration).
fn judging_sha256(
    d: &Descriptor,
    model: &ModelSpec,
    decision_runs: u32,
    policy: &Policy,
    thresholds: &crate::gate_rules::Thresholds,
) -> String {
    let mut text = format!(
        "harness:{}\nconfig:{}\nprompt:{}:{}\ncatalog:{}:{}\n",
        d.harness_version,
        d.config_sha256,
        d.prompt_version,
        d.prompt_sha256,
        d.catalog_version,
        d.catalog_sha256
    );
    for (name, t) in &d.tools {
        text.push_str(&format!(
            "tool:{name}:{}:{}\n",
            t.manifest_sha256,
            t.lock_digest.as_deref().unwrap_or("")
        ));
    }
    text.push_str(&format!(
        "model:{}\ndecision_runs:{decision_runs}\npolicy:{:?}:{:?}\nthresholds:{}\n",
        serde_json::to_string(model).unwrap_or_default(),
        policy.on_unsure,
        policy.on_error,
        serde_json::to_string(thresholds).unwrap_or_default()
    ));
    sha256_hex(text.as_bytes())
}

fn receipt_id(
    stage: GateStage,
    tool: &str,
    set: &str,
    identity: &str,
    results: &str,
    seal: &str,
) -> String {
    sha256_hex(format!("{stage}\n{tool}\n{set}\n{identity}\n{results}\n{seal}\n").as_bytes())[..16]
        .to_string()
}

/// Runs the gate.
pub fn gate(req: &Request) -> Result<GateReport> {
    let m = &req.matrix.matrix;
    let spec = m.gate.as_ref().ok_or_else(|| {
        gate_err("the matrix has no [matrix.gate] section; the seed, resamples and thresholds must be frozen in the matrix")
    })?;
    if !m.sets.iter().any(|s| s == req.set) {
        return Err(gate_err(format!(
            "the matrix does not name set {:?}",
            req.set
        )));
    }
    let x_arm = Arm::DX(req.tool.to_string());
    if !m.arms.contains(&Arm::C) || !m.arms.contains(&x_arm) {
        return Err(gate_err(format!(
            "the matrix must have arms C and {x_arm} to score tool {:?}",
            req.tool
        )));
    }
    let model = match (req.model, m.models.as_slice()) {
        (None, [one]) => one,
        (None, _) => {
            return Err(gate_err(
                "the matrix has several models; name one with --model <profile>:<model>",
            ));
        }
        (Some(name), models) => models
            .iter()
            .find(|ms| format!("{}:{}", ms.profile, ms.model) == name)
            .ok_or_else(|| gate_err(format!("the matrix has no model {name:?}")))?,
    };
    let model_name = format!("{}:{}", model.profile, model.model);

    // The set, its seal and the labels; and across the root, no family in two splits.
    crate::sets::check_families(req.evalsets_root)?;
    let set_dir = req.evalsets_root.join(req.set);
    verify_seal(&set_dir)?;
    let set = Set::load(&set_dir)?;
    let seal = Seal::load(&set_dir)?;
    let (label_bytes, labels) = match (&set.labels, &set.manifest.set.labels, req.labels) {
        (Some(labels), Some(rel), _) => {
            let bytes = std::fs::read(set_dir.join(rel)).map_err(|e| io(&set_dir.join(rel), e))?;
            (bytes, labels.clone())
        }
        (_, _, Some(path)) => {
            let bytes = std::fs::read(path).map_err(|e| io(path, e))?;
            let text = String::from_utf8(bytes.clone())
                .map_err(|e| Error::Gate(format!("{}: {e}", path.display())))?;
            let labels: Labels = toml::from_str(&text)
                .map_err(|e| Error::Gate(format!("{}: {e}", path.display())))?;
            set.check_labels(&labels)?;
            (bytes, labels)
        }
        (_, _, None) => {
            return Err(gate_err(format!(
                "set {} carries no labels; supply the owner's label file with --labels",
                req.set
            )));
        }
    };
    let labels_sha256 = sha256_hex(&label_bytes);
    if labels_sha256 != seal.seal.components.labels {
        return Err(gate_err(format!(
            "label-hash mismatch: the supplied labels hash to {labels_sha256}, the seal of {} binds {}",
            req.set, seal.seal.components.labels
        )));
    }

    // The stage's structural requirements, before any result is read.
    let items = set.manifest.item.len();
    let receipts = Receipt::load_all(req.evalsets_root)?;
    let initial_receipt = match req.stage {
        GateStage::Initial => {
            if items < MIN_INITIAL_ITEMS {
                return Err(gate_err(format!(
                    "the initial stage needs at least {MIN_INITIAL_ITEMS} items; {} has {items}",
                    req.set
                )));
            }
            None
        }
        GateStage::Confirmation => {
            let initial_id = req.initial_receipt.ok_or_else(|| {
                gate_err(
                    "the confirmation stage needs the initial stage's receipt (--initial-receipt)",
                )
            })?;
            let initial = receipts
                .get(initial_id)
                .ok_or_else(|| gate_err(format!("no receipt {initial_id:?}")))?;
            let b = &initial.receipt;
            if b.stage != GateStage::Initial
                || b.tool != req.tool
                || b.use_case != set.manifest.set.use_case
                || b.model != model_name
            {
                return Err(gate_err(format!(
                    "receipt {initial_id} is not an initial-stage receipt for tool {:?}, use case {:?} and model {model_name}",
                    req.tool, set.manifest.set.use_case
                )));
            }
            if b.outcome != Outcome::Admitted {
                return Err(gate_err(format!(
                    "receipt {initial_id} did not admit the tool (outcome {:?})",
                    b.outcome
                )));
            }
            if b.set == req.set {
                return Err(gate_err(
                    "the confirmation stage scores a set distinct from the initial stage's",
                ));
            }
            if items < MIN_CONFIRMATION_ITEMS {
                return Err(gate_err(format!(
                    "the confirmation stage needs at least {MIN_CONFIRMATION_ITEMS} items; {} has {items}",
                    req.set
                )));
            }
            // Families disjoint with every earlier set of the use case.
            let mine: BTreeSet<&str> = set.families.family.iter().map(|f| f.id.as_str()).collect();
            let mut earlier_sets: BTreeSet<&str> = receipts
                .values()
                .filter(|r| r.receipt.use_case == set.manifest.set.use_case)
                .map(|r| r.receipt.set.as_str())
                .collect();
            earlier_sets.remove(req.set);
            for other in earlier_sets {
                let other_set = Set::load(&req.evalsets_root.join(other))?;
                let overlap: Vec<&str> = other_set
                    .families
                    .family
                    .iter()
                    .map(|f| f.id.as_str())
                    .filter(|f| mine.contains(f))
                    .collect();
                if !overlap.is_empty() {
                    return Err(gate_err(format!(
                        "families {} of {} also appear in the earlier set {other}",
                        overlap.join(", "),
                        req.set
                    )));
                }
            }
            Some(initial_id.to_string())
        }
    };

    // The results against the descriptor and the frozen identity.
    let identity = req.matrix.identity();
    let descriptor = Descriptor::read(req.eval_dir)?;
    if descriptor.matrix_identity != identity {
        return Err(gate_err(format!(
            "descriptor mismatch: the results were produced under matrix identity {}, the given matrix has {identity}",
            descriptor.matrix_identity
        )));
    }
    match descriptor.sets.get(req.set) {
        Some(c) if *c == seal.seal.components => {}
        Some(_) => {
            return Err(gate_err(format!(
                "descriptor mismatch: the results were produced over a different sealing of {}",
                req.set
            )));
        }
        None => {
            return Err(gate_err(format!(
                "descriptor mismatch: the results do not cover set {}",
                req.set
            )));
        }
    }
    let summary_path = req.eval_dir.join(SUMMARY_FILE);
    let summary_bytes = std::fs::read(&summary_path).map_err(|e| io(&summary_path, e))?;
    let summary: Summary = serde_json::from_slice(&summary_bytes)
        .map_err(|e| Error::Gate(format!("{}: {e}", summary_path.display())))?;
    if summary.matrix_identity != identity || summary.descriptor_sha256 != descriptor.sha256 {
        return Err(gate_err(
            "descriptor mismatch: summary.json was written under another descriptor",
        ));
    }
    if summary.cancelled {
        return Err(gate_err(
            "the evaluation was cancelled; resume it with `eval run` before scoring",
        ));
    }
    if summary.budget_exhausted {
        return Err(gate_err(
            "the evaluation stopped at its spend cap; it holds no result for every item, so nothing can be scored",
        ));
    }
    // Over the decisions alone, in id order: what the run did (completed, skipped, spend) may
    // differ between a run and its no-op resume without a single result changing.
    let mut decisions = summary.decisions.clone();
    decisions.sort_by(|a, b| a.id.cmp(&b.id));
    let results_sha256 = sha256_hex(
        serde_json::to_string(&decisions)
            .map_err(gate_err)?
            .as_bytes(),
    );
    let id = receipt_id(
        req.stage,
        req.tool,
        req.set,
        &identity,
        &results_sha256,
        &seal.seal.sha256,
    );
    let path = receipt_path(req.evalsets_root, &id);

    let (receipt, resumed) = if let Some(existing) = receipts.get(&id) {
        (existing.clone(), true)
    } else {
        // Repeat scoring: the set has a receipt from another scoring.
        let earlier: Vec<&Receipt> = receipts
            .values()
            .filter(|r| r.receipt.set == req.set)
            .collect();
        if !earlier.is_empty() {
            let used: BTreeSet<&str> = earlier
                .iter()
                .filter_map(|r| r.receipt.adr.as_deref())
                .collect();
            match req.adr {
                None => {
                    return Err(gate_err(format!(
                        "set {} was already scored (receipt {}); scoring it again needs a new ADR reference (--adr)",
                        req.set, earlier[0].receipt.id
                    )));
                }
                Some(adr) if used.contains(adr) => {
                    return Err(gate_err(format!(
                        "ADR reference {adr:?} already permitted a scoring of {}; a new one is needed",
                        req.set
                    )));
                }
                Some(_) => {}
            }
        }
        if req.stage == GateStage::Initial && set.manifest.set.split != Split::Blind {
            return Err(gate_err(format!(
                "the initial stage scores a blind set; {} is {}",
                req.set, set.manifest.set.split
            )));
        }
        // Score.
        let mut by_set = BTreeMap::new();
        by_set.insert(req.set, &labels);
        let (rows, policy) =
            summary::rows(req.eval_dir, std::slice::from_ref(&set), &by_set, &summary)?;
        let c_cell = format!("{model_name}/C");
        let x_cell = format!("{model_name}/{x_arm}");
        let mut c_rows = BTreeMap::new();
        let mut x_rows = BTreeMap::new();
        for r in rows.iter().filter(|r| r.repetition == 1) {
            if r.cell == c_cell {
                c_rows.insert(r.item.as_str(), r);
            } else if r.cell == x_cell {
                x_rows.insert(r.item.as_str(), r);
            }
        }
        let family_of: BTreeMap<&str, &str> = set
            .manifest
            .item
            .iter()
            .map(|i| (i.id.as_str(), i.family.as_str()))
            .collect();
        let mut scored_c = Vec::new();
        let mut scored_x = Vec::new();
        let mut paired = Vec::new();
        let (mut cost_c, mut cost_x) = (0.0, 0.0);
        for (item, family) in &family_of {
            let (Some(c), Some(x)) = (c_rows.get(item), x_rows.get(item)) else {
                return Err(gate_err(format!(
                    "incomplete results: item {item} lacks a decision under {c_cell} or {x_cell}"
                )));
            };
            let expected_fail = c.expected == Expected::Fail;
            scored_c.push(Scored {
                expected_fail,
                computed: c.computed,
            });
            scored_x.push(Scored {
                expected_fail,
                computed: x.computed,
            });
            paired.push(PairedItem {
                family: family.to_string(),
                id: item.to_string(),
                expected_fail,
                detected_c: c.computed == Computed::Fail,
                detected_x: x.computed == Computed::Fail,
            });
            cost_c += c.cost_usd;
            cost_x += x.cost_usd;
        }
        let judging = judging_sha256(
            &descriptor,
            model,
            m.decision_runs,
            &policy,
            &spec.thresholds,
        );
        if let Some(initial_id) = req.initial_receipt
            && let Some(initial) = receipts.get(initial_id)
            && initial.receipt.judging_sha256 != judging
        {
            return Err(gate_err(format!(
                "the judging configuration differs from receipt {initial_id}'s; both stages must run under the identical frozen configuration"
            )));
        }
        let c = ArmMetrics::from_items(&scored_c, &policy, cost_c);
        let x = ArmMetrics::from_items(&scored_x, &policy, cost_x);
        let boot = bootstrap::paired_bootstrap(
            &paired,
            spec.seed,
            spec.resamples,
            req.matrix.hypotheses(),
        )?;
        let verdict = decide(&c, &x, &boot, &spec.thresholds);
        let receipt = Receipt {
            receipt: ReceiptBody {
                receipt_version: RECEIPT_VERSION,
                id: id.clone(),
                created: crate::sets::utc_now(),
                stage: req.stage,
                tool: req.tool.to_string(),
                use_case: set.manifest.set.use_case.clone(),
                set: req.set.to_string(),
                model: model_name.clone(),
                items,
                matrix_identity: identity,
                descriptor_sha256: descriptor.sha256.clone(),
                seal_sha256: seal.seal.sha256.clone(),
                labels_sha256,
                results_sha256,
                judging_sha256: judging,
                adr: req.adr.map(str::to_string),
                initial_receipt: initial_receipt.clone(),
                outcome: verdict.outcome,
                failed: verdict.failed,
                c,
                x,
                bootstrap: boot,
            },
        };
        let dir = req.evalsets_root.join(RECEIPTS_DIR);
        std::fs::create_dir_all(&dir).map_err(|e| io(&dir, e))?;
        let text = toml::to_string_pretty(&receipt).map_err(gate_err)?;
        crate::write_atomic(&path, text.as_bytes()).map_err(|e| io(&path, e))?;
        (receipt, false)
    };
    let mut written = vec![path];

    // Publication: a blind set's labels join the set and it becomes heldout. Idempotent, so a
    // resumed gate redoes what an interrupted one left undone.
    if set.manifest.set.split == Split::Blind {
        let labels_path = set_dir.join(PUBLISHED_LABELS);
        std::fs::write(&labels_path, &label_bytes).map_err(|e| io(&labels_path, e))?;
        let manifest_path = set_dir.join(MANIFEST_FILE);
        publish_manifest(&manifest_path)?;
        let republished = Set::load(&set_dir)?;
        republished.check()?;
        verify_seal(&set_dir)?;
        written.push(labels_path);
        written.push(manifest_path);
    }

    // Admission state.
    let mut admissions = if req.admissions.exists() {
        Admissions::load(req.admissions)?
    } else {
        Admissions::default()
    };
    let use_case = set.manifest.set.use_case.clone();
    let mut admission = admissions.stage(req.tool, &use_case);
    if receipt.receipt.outcome == Outcome::Admitted {
        let stage = match req.stage {
            GateStage::Initial => Stage::Provisional,
            GateStage::Confirmation => Stage::Confirmed,
        };
        // An initial stage never demotes a confirmed tool.
        if !(stage == Stage::Provisional && admission == Some(Stage::Confirmed)) {
            admissions
                .admission
                .retain(|a| !(a.tool == req.tool && a.use_case == use_case));
            admissions.admission.push(Admission {
                tool: req.tool.to_string(),
                use_case: use_case.clone(),
                stage,
                receipt: receipt.receipt.id.clone(),
            });
            admissions.admission.sort_by(|a, b| {
                a.tool
                    .cmp(&b.tool)
                    .then_with(|| a.use_case.cmp(&b.use_case))
            });
            admissions.write(req.admissions)?;
            written.push(req.admissions.to_path_buf());
            admission = Some(stage);
        }
    }
    Ok(GateReport {
        receipt,
        resumed,
        written,
        admission,
    })
}

/// Rewrites a blind manifest for publication: `labels_sha256 = "…"` becomes
/// `labels = "labels.toml"` and `split = "blind"` becomes `split = "heldout"`, the rest of the
/// hand-written file untouched. A manifest already published is left as it is.
fn publish_manifest(path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(path).map_err(|e| io(path, e))?;
    let out: Vec<String> = text
        .lines()
        .map(|line| {
            let t = line.trim_start();
            if t.starts_with("labels_sha256 ") || t.starts_with("labels_sha256=") {
                format!("labels = {PUBLISHED_LABELS:?}")
            } else if t.starts_with("split ") || t.starts_with("split=") {
                "split = \"heldout\"".to_string()
            } else {
                line.to_string()
            }
        })
        .collect();
    let new = out.join("\n") + "\n";
    if new != text {
        std::fs::write(path, new).map_err(|e| io(path, e))?;
    }
    Ok(())
}
