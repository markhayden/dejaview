//! The execution descriptor (SPEC "Runner and arms"): every answer-affecting input of an
//! evaluation, resolved and hashed before the first decision, written to the evaluation
//! directory. A resume recomputes it and refuses on any difference. The single-writer lock
//! keeps two processes out of one evaluation directory.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use dejaview_core::image::sha256_hex;
use serde::{Deserialize, Serialize};

use crate::matrix::Matrix;
use crate::sets::{Components, Set};
use crate::{Error, Result};

pub const DESCRIPTOR_FILE: &str = "descriptor.json";
pub const LOCK_FILE: &str = "LOCK";
pub const DESCRIPTOR_VERSION: u32 = 1;

/// What the descriptor is computed from.
pub struct Inputs {
    pub matrix: Matrix,
    pub matrix_text: String,
    /// The base config file's text.
    pub config_text: String,
    pub evalsets_root: PathBuf,
    /// (tool name, frozen manifest as JSON, pack lock digest if any).
    pub tools: Vec<(String, String, Option<String>)>,
    /// Every arm's tool selection per use case (`<arm>@<use case>` -> tools in prompt order,
    /// `tool:projection` where projected), so a changed admission record cannot change arm D
    /// between runs of one evaluation.
    pub selections: BTreeMap<String, Vec<String>>,
    pub harness_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub descriptor_version: u32,
    pub harness_version: String,
    pub matrix_identity: String,
    pub matrix_sha256: String,
    pub config_sha256: String,
    pub prompt_version: String,
    pub prompt_sha256: String,
    pub catalog_version: u32,
    pub catalog_sha256: String,
    /// The profiles the matrix's models use (their definitions are in the catalog).
    pub profiles: Vec<String>,
    pub sets: BTreeMap<String, Components>,
    pub tools: BTreeMap<String, ToolDigest>,
    /// Every arm's tool selection per use case (see [`Inputs::selections`]).
    #[serde(default)]
    pub selections: BTreeMap<String, Vec<String>>,
    /// Over every field above.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDigest {
    pub manifest_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock_digest: Option<String>,
}

impl Descriptor {
    pub fn compute(inputs: &Inputs) -> Result<Descriptor> {
        let mut sets = BTreeMap::new();
        for id in &inputs.matrix.matrix.sets {
            let set = Set::load(&inputs.evalsets_root.join(id))?;
            sets.insert(id.clone(), set.components()?);
        }
        let mut tools = BTreeMap::new();
        for (name, manifest, lock) in &inputs.tools {
            tools.insert(
                name.clone(),
                ToolDigest {
                    manifest_sha256: sha256_hex(manifest.as_bytes()),
                    lock_digest: lock.clone(),
                },
            );
        }
        let mut profiles: Vec<String> = inputs
            .matrix
            .matrix
            .models
            .iter()
            .map(|m| m.profile.clone())
            .collect();
        profiles.sort();
        profiles.dedup();
        let mut d = Descriptor {
            descriptor_version: DESCRIPTOR_VERSION,
            harness_version: inputs.harness_version.clone(),
            matrix_identity: inputs.matrix.identity(),
            matrix_sha256: sha256_hex(inputs.matrix_text.as_bytes()),
            config_sha256: sha256_hex(inputs.config_text.as_bytes()),
            prompt_version: dejaview_core::render::PROMPT_VERSION.to_string(),
            prompt_sha256: sha256_hex(dejaview_core::render::TEMPLATE.as_bytes()),
            catalog_version: dejaview_providers::catalog::CATALOG_VERSION,
            catalog_sha256: sha256_hex(dejaview_providers::catalog::SHIPPED.as_bytes()),
            profiles,
            sets,
            tools,
            selections: inputs.selections.clone(),
            sha256: String::new(),
        };
        d.sha256 = d.overall();
        Ok(d)
    }

    fn overall(&self) -> String {
        let mut copy = self.clone();
        copy.sha256.clear();
        sha256_hex(serde_json::to_string(&copy).unwrap_or_default().as_bytes())
    }

    pub fn write(&self, dir: &Path) -> Result<()> {
        let path = dir.join(DESCRIPTOR_FILE);
        let text =
            serde_json::to_string_pretty(self).map_err(|e| Error::Descriptor(e.to_string()))?;
        crate::write_atomic(&path, text.as_bytes())
            .map_err(|e| Error::Descriptor(format!("cannot write {}: {e}", path.display())))
    }

    pub fn read(dir: &Path) -> Result<Descriptor> {
        let path = dir.join(DESCRIPTOR_FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| Error::Descriptor(format!("cannot read {}: {e}", path.display())))?;
        let d: Descriptor = serde_json::from_str(&text)
            .map_err(|e| Error::Descriptor(format!("{}: {e}", path.display())))?;
        if d.sha256 != d.overall() {
            return Err(Error::Descriptor(format!(
                "{}: its sha256 does not match its content",
                path.display()
            )));
        }
        Ok(d)
    }

    /// Refuses to resume an evaluation whose stored descriptor differs from `now`, naming the
    /// first inputs that changed.
    pub fn check_resume(dir: &Path, now: &Descriptor) -> Result<()> {
        let stored = Descriptor::read(dir)?;
        if stored == *now {
            return Ok(());
        }
        let mut changed = Vec::new();
        if stored.descriptor_version != now.descriptor_version {
            changed.push("descriptor version".to_string());
        }
        if stored.harness_version != now.harness_version {
            changed.push(format!(
                "harness version ({} now {})",
                stored.harness_version, now.harness_version
            ));
        }
        if stored.matrix_identity != now.matrix_identity
            || stored.matrix_sha256 != now.matrix_sha256
        {
            changed.push("matrix".to_string());
        }
        if stored.config_sha256 != now.config_sha256 {
            changed.push("config".to_string());
        }
        if stored.prompt_version != now.prompt_version || stored.prompt_sha256 != now.prompt_sha256
        {
            changed.push("prompt".to_string());
        }
        if stored.catalog_version != now.catalog_version
            || stored.catalog_sha256 != now.catalog_sha256
        {
            changed.push("catalog".to_string());
        }
        if stored.profiles != now.profiles {
            changed.push("profiles".to_string());
        }
        if stored.selections != now.selections {
            changed.push("arm selections (tools per arm and use case)".to_string());
        }
        for (id, a) in &stored.sets {
            match now.sets.get(id) {
                None => changed.push(format!("set {id} (gone)")),
                Some(b) if a != b => {
                    let mut parts = Vec::new();
                    for (name, x, y) in [
                        ("items", &a.items, &b.items),
                        ("families", &a.families, &b.families),
                        ("jobs", &a.jobs, &b.jobs),
                        ("images", &a.images, &b.images),
                        ("rules", &a.rules, &b.rules),
                        ("labels", &a.labels, &b.labels),
                    ] {
                        if x != y {
                            parts.push(name);
                        }
                    }
                    changed.push(format!("set {id}: {}", parts.join(", ")));
                }
                Some(_) => {}
            }
        }
        for id in now.sets.keys() {
            if !stored.sets.contains_key(id) {
                changed.push(format!("set {id} (new)"));
            }
        }
        for (name, a) in &stored.tools {
            match now.tools.get(name) {
                None => changed.push(format!("tool {name} (gone)")),
                Some(b) if a != b => changed.push(format!("tool {name}")),
                Some(_) => {}
            }
        }
        for name in now.tools.keys() {
            if !stored.tools.contains_key(name) {
                changed.push(format!("tool {name} (new)"));
            }
        }
        Err(Error::Descriptor(format!(
            "the evaluation in {} was started with different inputs; changed: {}. Start a new evaluation directory.",
            dir.display(),
            changed.join("; ")
        )))
    }
}

/// The single-writer lock on an evaluation directory: a `LOCK` file naming the writer's pid,
/// created exclusively, removed when the lock is dropped. A lock whose process no longer exists
/// is stale and taken over.
#[derive(Debug)]
pub struct WriterLock {
    /// Held open: the OS advisory lock on it is the lock, released when the process ends however
    /// it ends, so a dead writer never leaves a stale one and two writers never both succeed.
    _file: std::fs::File,
}

impl WriterLock {
    pub fn acquire(dir: &Path) -> Result<WriterLock> {
        std::fs::create_dir_all(dir)
            .map_err(|e| Error::Descriptor(format!("cannot create {}: {e}", dir.display())))?;
        let path = dir.join(LOCK_FILE);
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| Error::Descriptor(format!("cannot open {}: {e}", path.display())))?;
        match file.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                let holder = std::fs::read_to_string(&path).unwrap_or_default();
                return Err(Error::Descriptor(format!(
                    "{} is held by another writer ({}); one evaluation directory accepts one writer at a time",
                    dir.display(),
                    holder.trim()
                )));
            }
            Err(std::fs::TryLockError::Error(e)) => {
                return Err(Error::Descriptor(format!(
                    "cannot lock {}: {e}",
                    path.display()
                )));
            }
        }
        let _ = file.set_len(0);
        let _ = writeln!(file, "pid = {}", std::process::id());
        let _ = file.sync_data();
        Ok(WriterLock { _file: file })
    }
}
