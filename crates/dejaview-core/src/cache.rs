//! The decision cache (SPEC "Cache key"). The key covers everything that determines a decision
//! and nothing else: the resolved job and its images, the merged rules and instances, the rendered
//! prompt, the frozen tool manifests, every schema version, the adapter, profile and model with
//! their controls, every budget and image setting, `decision_runs`, and the harness version.
//! Credentials, the output directory, the cache settings and timestamps are excluded.
//!
//! The cache is consulted once per decision, before the first run, and written once after the last
//! run if none errored. Entries point at decision directories; a hit whose report or artifacts have
//! been pruned is a miss. Caching is off until `[cache] dir` is set.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{
    BudgetConfig, CONFIG_VERSION, CacheConfig, ImagesConfig, RulesConfig, ToolsConfig,
};
use crate::image::{CanonicalImage, hex};
use crate::job::{Job, Projection, Role};
use crate::obligations::Instance;
use crate::report::{REPORT_VERSION, Report};
use crate::run::{Prepared, RulesSummary};
use crate::submission::SUBMISSION_SCHEMA_VERSION;
use crate::tool::envelope::ENVELOPE_VERSION;
use crate::tool::manifest::MANIFEST_VERSION;
use crate::tool::{Manifest, ToolRegistry};
use crate::{Error, Result};

/// Everything the key is computed over, in a fixed serialisation.
#[derive(Debug, Clone, Serialize)]
pub struct KeyInput {
    pub use_case: String,
    pub brief: Option<String>,
    pub images: Vec<KeyImage>,
    pub rules: RulesSummary,
    pub instances: Vec<Instance>,
    pub checklist: Vec<String>,
    pub system_prompt: String,
    pub tools: Vec<Manifest>,
    pub versions: BTreeMap<String, u32>,
    pub adapter: String,
    pub profile: String,
    pub model: String,
    pub controls: BTreeMap<String, serde_json::Value>,
    pub max_output_tokens: u32,
    pub budget: BudgetConfig,
    pub image_settings: ImagesConfig,
    pub tool_settings: ToolsConfig,
    pub rules_settings: Option<RulesConfig>,
    pub harness_version: String,
}

/// One image as the key sees it.
#[derive(Debug, Clone, Serialize)]
pub struct KeyImage {
    pub id: String,
    pub role: Role,
    pub label: Option<String>,
    pub projection: Projection,
    pub sha256: String,
    pub canonical_sha256: String,
    pub prep_version: String,
}

/// The preparation the images went through (task 2.3 adds per-model preparation).
pub const PREP_VERSION: &str = "canonical";

impl KeyInput {
    /// Gathers the key inputs from a resolved decision.
    pub fn gather(
        config: &crate::config::Config,
        job: &Job,
        images: &[CanonicalImage],
        prepared: &Prepared,
        tools: &ToolRegistry,
        adapter: &str,
        harness_version: &str,
    ) -> KeyInput {
        KeyInput {
            use_case: job.use_case.clone(),
            brief: job.brief.clone(),
            images: images
                .iter()
                .map(|i| KeyImage {
                    id: i.id.clone(),
                    role: i.role,
                    label: i.label.clone(),
                    projection: i.projection,
                    sha256: i.sha256.clone(),
                    canonical_sha256: i.canonical_sha256.clone(),
                    prep_version: PREP_VERSION.into(),
                })
                .collect(),
            rules: prepared.summary.clone(),
            instances: prepared.resolved.instances.clone(),
            checklist: prepared.checklist.ids().map(str::to_string).collect(),
            system_prompt: prepared.system_prompt.clone(),
            tools: tools.manifests().cloned().collect(),
            versions: BTreeMap::from([
                ("config".to_string(), CONFIG_VERSION),
                ("manifest".to_string(), MANIFEST_VERSION),
                ("envelope".to_string(), ENVELOPE_VERSION),
                ("submission".to_string(), SUBMISSION_SCHEMA_VERSION),
                ("report".to_string(), REPORT_VERSION),
            ]),
            adapter: adapter.to_string(),
            profile: config.provider.profile.clone(),
            model: config.provider.model.clone(),
            controls: config.provider.controls.clone(),
            max_output_tokens: config.provider.max_output_tokens,
            budget: config.budget.clone(),
            image_settings: config.images.clone(),
            tool_settings: config.tools.clone(),
            rules_settings: config.rules.clone(),
            harness_version: harness_version.to_string(),
        }
    }

    /// The key: SHA-256 of the canonical JSON of the inputs.
    pub fn key(&self) -> Result<String> {
        let bytes = serde_json::to_vec(self)?;
        Ok(hex(&Sha256::digest(&bytes)))
    }
}

/// How the cache figured in a decision, as the header and report record it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct CacheStatus {
    /// `off` (no `[cache] dir`, or the rules did not resolve), `miss`, `hit` or `bypass`
    /// (an evaluation, which never reads).
    pub status: String,
    pub key: Option<String>,
    /// On a hit: the decision directory the result came from.
    pub source: Option<PathBuf>,
}

impl CacheStatus {
    pub fn off() -> CacheStatus {
        CacheStatus {
            status: "off".into(),
            key: None,
            source: None,
        }
    }

    pub fn is_hit(&self) -> bool {
        self.status == "hit"
    }
}

/// A cache entry: a decision by reference to its directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub key: String,
    pub decision_dir: PathBuf,
    pub report_version: u32,
    pub created_unix_ms: u64,
}

/// The on-disk cache: one JSON entry per key under `<dir>/decisions/`.
#[derive(Debug)]
pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    /// Opens the cache the config names; `None` when caching is off (no `dir`).
    pub fn from_config(config: &CacheConfig) -> Result<Option<Cache>> {
        let Some(dir) = &config.dir else {
            return Ok(None);
        };
        Ok(Some(Cache::open(&expand_home(dir))?))
    }

    pub fn open(dir: &Path) -> Result<Cache> {
        std::fs::create_dir_all(dir.join("decisions"))?;
        Ok(Cache {
            dir: dir.to_path_buf(),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn entry_path(&self, key: &str) -> PathBuf {
        self.dir.join("decisions").join(format!("{key}.json"))
    }

    /// The entry for a key whose decision directory still holds its report and every artifact the
    /// report references. A stale entry is removed and reported as a miss.
    pub fn lookup(&self, key: &str) -> Result<Option<Entry>> {
        let path = self.entry_path(key);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let entry: Entry = match serde_json::from_str(&text) {
            Ok(e) => e,
            Err(_) => {
                std::fs::remove_file(&path)?;
                return Ok(None);
            }
        };
        if entry.report_version != REPORT_VERSION || !intact(&entry.decision_dir) {
            std::fs::remove_file(&path)?;
            return Ok(None);
        }
        Ok(Some(entry))
    }

    /// Records a decision for a key.
    pub fn store(&self, key: &str, decision_dir: &Path) -> Result<Entry> {
        let decision_dir = decision_dir.canonicalize().map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", decision_dir.display()),
            ))
        })?;
        let entry = Entry {
            key: key.to_string(),
            decision_dir,
            report_version: REPORT_VERSION,
            created_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        };
        let path = self.entry_path(key);
        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&tmp, serde_json::to_string_pretty(&entry)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(entry)
    }
}

/// Whether a decision directory still holds its report and every artifact the report references.
pub fn intact(decision_dir: &Path) -> bool {
    let Ok(report) = Report::read(decision_dir) else {
        return false;
    };
    report.runs.iter().all(|run| {
        let artifacts = decision_dir
            .join(format!("run-{}", run.run))
            .join("artifacts");
        let present = |r: &crate::artifacts::ArtifactRef| artifacts.join(&r.sha256).is_file();
        run.images_sent.values().all(present)
            && run
                .turns
                .iter()
                .all(|t| present(&t.request) && t.response.as_ref().is_none_or(present))
            && run
                .tool_calls
                .iter()
                .all(|c| c.envelope.as_ref().is_none_or(present))
    })
}

/// `~` and `~/...` resolve against `HOME`.
pub fn expand_home(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    if (text == "~" || text.starts_with("~/"))
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(text.trim_start_matches('~').trim_start_matches('/'));
    }
    path.to_path_buf()
}
