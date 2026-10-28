//! Evaluation sets (SPEC "Set manifest", "Seal"): the manifest, families and labels of one set,
//! `check` over a tree of sets, and the seal that binds a set's content.
//!
//! An evaluation item is a complete job plus labels. The job is canonical and runnable by
//! `dejaview check --job`; labels live in a separate file so blind sets ship without them. Image
//! paths in an item's job are relative to the set directory and every image is pinned by SHA-256.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use dejaview_core::image::sha256_hex;
use dejaview_core::job::{ImageSource, Job};
use dejaview_core::obligations::{self, MergeInput, RulesFile};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const MANIFEST_FILE: &str = "set.toml";
pub const SEAL_FILE: &str = "seal.toml";

/// `set.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub set: SetMeta,
    #[serde(default)]
    pub item: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetMeta {
    /// `<use-case>/<set>`, which is also the set's path under the evalsets root.
    pub id: String,
    pub use_case: String,
    pub split: Split,
    pub licence: String,
    /// Where the images live: `github.com/<owner>/<repo>` (fetched by hash from the pinned
    /// revision), an `https://` base URL, or a directory (`.` for images beside the manifest).
    pub images_repo: String,
    /// The revision of `images_repo` the images are fetched from; required for a remote repo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images_rev: Option<String>,
    /// The label file, relative to the set directory; absent for a blind set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<PathBuf>,
    /// SHA-256 of the label file; present for a blind set, whose labels are held elsewhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels_sha256: Option<String>,
    pub families: PathBuf,
    pub rules: RulesPin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesPin {
    pub base: String,
    pub version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Split {
    Dev,
    Heldout,
    Blind,
}

impl std::fmt::Display for Split {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Split::Dev => "dev",
            Split::Heldout => "heldout",
            Split::Blind => "blind",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub family: String,
    /// The job file, relative to the set directory.
    pub job: PathBuf,
}

/// `families.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Families {
    #[serde(default)]
    pub family: Vec<Family>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    pub id: String,
    pub description: String,
}

/// `labels.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Labels {
    #[serde(default)]
    pub label: Vec<Label>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Label {
    pub item: String,
    pub expected: Expected,
    #[serde(default)]
    pub failure_types: Vec<String>,
    pub words: String,
    #[serde(default)]
    pub obligation_ids: Vec<String>,
    pub adjudicated_by: String,
    /// YYYY-MM-DD.
    pub adjudicated_on: String,
    #[serde(default)]
    pub regions: Vec<LabelRegion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Expected {
    Pass,
    Fail,
}

/// A labelled region on a root image, in root coordinates (SPEC "Metrics", localisation).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelRegion {
    pub frame: String,
    pub kind: RegionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<Vec<[f64; 2]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    pub defect: String,
    #[serde(default)]
    pub obligation_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionKind {
    Polygon,
    Bbox,
    WholeImage,
    Absent,
}

/// One set as loaded from its directory: manifest, families, labels where present, and every
/// item's job.
#[derive(Debug, Clone)]
pub struct Set {
    pub dir: PathBuf,
    pub manifest: Manifest,
    pub families: Families,
    pub labels: Option<Labels>,
    /// Jobs by item id, in manifest order.
    pub jobs: Vec<(String, Job)>,
}

fn sets_err(set: &str, what: impl std::fmt::Display) -> Error {
    Error::Sets(format!("{set}: {what}"))
}

fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Error::Sets(format!("cannot read {}: {e}", path.display())))?;
    toml::from_str(&text).map_err(|e| Error::Sets(format!("{}: {e}", path.display())))
}

impl Set {
    /// Loads a set from its directory, parsing the manifest, families, labels and every job.
    /// Parsing is strict (unknown keys are refused), so an answer label in a manifest or job is a
    /// load error.
    pub fn load(dir: &Path) -> Result<Set> {
        let manifest: Manifest = read_toml(&dir.join(MANIFEST_FILE))?;
        let id = manifest.set.id.clone();
        let families: Families = read_toml(&dir.join(&manifest.set.families))?;
        let labels = match &manifest.set.labels {
            Some(rel) => Some(read_toml::<Labels>(&dir.join(rel))?),
            None => None,
        };
        let mut jobs = Vec::new();
        for item in &manifest.item {
            let job = Job::load(&dir.join(&item.job))
                .map_err(|e| sets_err(&id, format!("item {}: {e}", item.id)))?;
            jobs.push((item.id.clone(), job));
        }
        Ok(Set {
            dir: dir.to_path_buf(),
            manifest,
            families,
            labels,
            jobs,
        })
    }

    /// Every image of every item: item id, image id, path relative to the set directory, and the
    /// pinned hash.
    pub fn images(&self) -> Result<Vec<SetImage>> {
        let id = &self.manifest.set.id;
        let mut out = Vec::new();
        for (item, job) in &self.jobs {
            for spec in &job.images {
                match &spec.source {
                    ImageSource::Path {
                        path,
                        sha256: Some(sha),
                    } => out.push(SetImage {
                        item: item.clone(),
                        image: spec.id.clone(),
                        path: path.clone(),
                        sha256: sha.to_ascii_lowercase(),
                    }),
                    ImageSource::Path { sha256: None, .. } => {
                        return Err(sets_err(
                            id,
                            format!("item {item}: image {} has no sha256 pin", spec.id),
                        ));
                    }
                    ImageSource::Base64 { .. } => {
                        return Err(sets_err(
                            id,
                            format!(
                                "item {item}: image {} is inline; set images are files",
                                spec.id
                            ),
                        ));
                    }
                }
            }
        }
        Ok(out)
    }

    /// The shipped rules the set pins, checked for use case and version.
    pub fn rules(&self) -> Result<RulesFile> {
        let id = &self.manifest.set.id;
        let pin = &self.manifest.set.rules;
        let text = obligations::shipped(&pin.base)
            .ok_or_else(|| sets_err(id, format!("rules base {:?} is not shipped", pin.base)))?;
        let file = RulesFile::parse(text, &pin.base).map_err(|e| sets_err(id, e))?;
        if file.rules.version != pin.version {
            return Err(sets_err(
                id,
                format!(
                    "rules version {} is pinned but the shipped {} base is version {}",
                    pin.version, pin.base, file.rules.version
                ),
            ));
        }
        if file.rules.use_case != self.manifest.set.use_case {
            return Err(sets_err(
                id,
                format!(
                    "use case {:?} but the pinned rules base {} is for {:?}",
                    self.manifest.set.use_case, pin.base, file.rules.use_case
                ),
            ));
        }
        Ok(file)
    }

    /// Every rule of SPEC "Set manifest" that one set can break on its own.
    pub fn check(&self) -> Result<SetReport> {
        let id = &self.manifest.set.id;
        let meta = &self.manifest.set;
        let rules = self.rules()?;
        // Items: unique ids, declared families, jobs of the set's use case that resolve.
        let mut seen = BTreeMap::new();
        for ((item, job), entry) in self.jobs.iter().zip(&self.manifest.item) {
            if seen.insert(item.clone(), ()).is_some() {
                return Err(sets_err(id, format!("item {item} appears twice")));
            }
            if !self.families.family.iter().any(|f| f.id == entry.family) {
                return Err(sets_err(
                    id,
                    format!(
                        "item {item}: family {:?} is not declared in {}",
                        entry.family,
                        meta.families.display()
                    ),
                ));
            }
            if job.use_case != meta.use_case {
                return Err(sets_err(
                    id,
                    format!(
                        "item {item}: job use case {:?} differs from the set's {:?}",
                        job.use_case, meta.use_case
                    ),
                ));
            }
            let merged = obligations::merge(&MergeInput {
                base: &rules.rules.use_case,
                profile: None,
                extra: &[],
                extra_names: &[],
                job: Some(job),
                enabled_tools: &[],
            })
            .map_err(|e| sets_err(id, format!("item {item}: {e}")))?;
            obligations::resolve(&merged, job)
                .map_err(|e| sets_err(id, format!("item {item}: {e}")))?;
        }
        // Images: present and matching their pins.
        let images = self.images()?;
        for img in &images {
            let full = self.dir.join(&img.path);
            let bytes = std::fs::read(&full).map_err(|e| {
                sets_err(
                    id,
                    format!(
                        "item {}: image {} ({}) cannot be read: {e}; run `dejaview sets fetch`",
                        img.item,
                        img.image,
                        img.path.display()
                    ),
                )
            })?;
            let actual = sha256_hex(&bytes);
            if actual != img.sha256 {
                return Err(sets_err(
                    id,
                    format!(
                        "item {}: image {} ({}) has sha256 {actual}, the job pins {}",
                        img.item,
                        img.image,
                        img.path.display(),
                        img.sha256
                    ),
                ));
            }
        }
        // Labels per split.
        match (meta.split, &self.labels, &meta.labels_sha256) {
            (Split::Blind, Some(_), _) => {
                return Err(sets_err(
                    id,
                    "a blind set carries no labels file; seal it and move the labels out",
                ));
            }
            (Split::Blind, None, None) => {
                return Err(sets_err(id, "a blind set must carry labels_sha256"));
            }
            (Split::Blind, None, Some(_)) => {}
            (split, None, _) => {
                return Err(sets_err(
                    id,
                    format!("a {split} set must name its labels file (labels.toml)"),
                ));
            }
            (_, Some(labels), _) => self.check_labels(labels)?,
        }
        Ok(SetReport {
            items: self.jobs.len(),
            images: images.len(),
        })
    }

    /// Labels for this set: every item labelled exactly once, nothing else labelled, every
    /// region with the geometry of its kind. Used for the set's own label file and for the
    /// labels supplied to a gate on a blind set.
    pub fn check_labels(&self, labels: &Labels) -> Result<()> {
        let id = &self.manifest.set.id;
        let items: BTreeSet<&str> = self.jobs.iter().map(|(i, _)| i.as_str()).collect();
        let mut labelled = BTreeSet::new();
        for l in &labels.label {
            if !items.contains(l.item.as_str()) {
                return Err(sets_err(
                    id,
                    format!("label for item {:?}, which the set does not have", l.item),
                ));
            }
            if !labelled.insert(l.item.as_str()) {
                return Err(sets_err(id, format!("item {} is labelled twice", l.item)));
            }
            for r in &l.regions {
                match (r.kind, &r.polygon, &r.bbox) {
                    (RegionKind::Polygon, Some(p), None) if p.len() >= 3 => {}
                    (RegionKind::Bbox, None, Some(_)) => {}
                    (RegionKind::WholeImage | RegionKind::Absent, None, None) => {}
                    _ => {
                        return Err(sets_err(
                            id,
                            format!(
                                "item {}: region of kind {:?} with the wrong geometry",
                                l.item, r.kind
                            ),
                        ));
                    }
                }
            }
        }
        for item in &items {
            if !labelled.contains(item) {
                return Err(sets_err(id, format!("item {item} has no label")));
            }
        }
        Ok(())
    }
}

/// One image of a set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetImage {
    pub item: String,
    pub image: String,
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SetReport {
    pub items: usize,
    pub images: usize,
}

/// What `check` over a tree found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CheckReport {
    pub sets: usize,
    pub items: usize,
    pub images: usize,
}

/// Every `set.toml` under a root (`<use-case>/<set>/set.toml`), or the root itself when it is a
/// set directory.
pub fn discover(root: &Path) -> Result<Vec<PathBuf>> {
    if root.join(MANIFEST_FILE).is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    let mut found = Vec::new();
    let read = |dir: &Path| -> Result<Vec<PathBuf>> {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| Error::Sets(format!("cannot read {}: {e}", dir.display())))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        Ok(dirs)
    };
    for use_case in read(root)? {
        for set in read(&use_case)? {
            if set.join(MANIFEST_FILE).is_file() {
                found.push(set);
            }
        }
    }
    if found.is_empty() {
        return Err(Error::Sets(format!(
            "no set.toml under {} (expected <use-case>/<set>/set.toml)",
            root.display()
        )));
    }
    Ok(found)
}

/// The cross-set rule alone, from the manifests: a family of one use case appears in a single
/// split. Needs no images, so a gate can apply it over a root whose images are not all fetched.
pub fn check_families(root: &Path) -> Result<()> {
    let mut families: BTreeMap<(String, String), (Split, String)> = BTreeMap::new();
    for dir in discover(root)? {
        let manifest: Manifest = read_toml(&dir.join(MANIFEST_FILE))?;
        let meta = &manifest.set;
        for item in &manifest.item {
            let key = (meta.use_case.clone(), item.family.clone());
            match families.get(&key) {
                Some((split, other)) if *split != meta.split => {
                    return Err(sets_err(
                        &meta.id,
                        format!(
                            "family {:?} is in the {} split here and in the {split} split of {other}; a family never appears in more than one split",
                            item.family, meta.split
                        ),
                    ));
                }
                _ => {
                    families.insert(key, (meta.split, meta.id.clone()));
                }
            }
        }
    }
    Ok(())
}

/// `dejaview sets check`: every set under the root passes its own checks, and across sets a
/// family of one use case appears in a single split.
pub fn check(root: &Path) -> Result<CheckReport> {
    let mut report = CheckReport::default();
    // (use case, family) -> (split, set id)
    let mut families: BTreeMap<(String, String), (Split, String)> = BTreeMap::new();
    for dir in discover(root)? {
        let set = Set::load(&dir)?;
        let r = set.check()?;
        let meta = &set.manifest.set;
        let expected_dir = root.join(&meta.id);
        if root.join(MANIFEST_FILE).is_file() {
            // A single set given by path: its id is not checked against the tree.
        } else if expected_dir != dir {
            return Err(sets_err(
                &meta.id,
                format!("the set lives at {}, not at its id", dir.display()),
            ));
        }
        for item in &set.manifest.item {
            let key = (meta.use_case.clone(), item.family.clone());
            match families.get(&key) {
                Some((split, other)) if *split != meta.split => {
                    return Err(sets_err(
                        &meta.id,
                        format!(
                            "family {:?} is in the {} split here and in the {split} split of {other}; a family never appears in more than one split",
                            item.family, meta.split
                        ),
                    ));
                }
                _ => {
                    families.insert(key, (meta.split, meta.id.clone()));
                }
            }
        }
        report.sets += 1;
        report.items += r.items;
        report.images += r.images;
    }
    Ok(report)
}

// ---- seal ----------------------------------------------------------------------------------

/// `seal.toml`: a SHA-256 over the set's content (SPEC "Seal"), by component so a verification
/// failure names what changed, plus the time of sealing and the owner's attestation that no judge
/// or tool had seen the items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seal {
    pub seal: SealBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealBody {
    /// Over the components, in the order of `Components`.
    pub sha256: String,
    /// UTC, `YYYY-MM-DDTHH:MM:SSZ`; outside the hash.
    pub created: String,
    /// The owner's words; outside the hash.
    pub attestation: String,
    pub components: Components,
}

/// One SHA-256 per component of the seal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Components {
    /// The sorted item ids.
    pub items: String,
    /// The sorted `item=family` assignments.
    pub families: String,
    /// Every item's job in canonical form, in item order.
    pub jobs: String,
    /// Every image file's hash, by path.
    pub images: String,
    /// The pinned rules base: name, version and content.
    pub rules: String,
    /// The label file (or, for a blind set, the hash the manifest carries).
    pub labels: String,
}

impl Components {
    fn overall(&self) -> String {
        sha256_hex(
            format!(
                "items:{}\nfamilies:{}\njobs:{}\nimages:{}\nrules:{}\nlabels:{}\n",
                self.items, self.families, self.jobs, self.images, self.rules, self.labels
            )
            .as_bytes(),
        )
    }

    /// The names of the components that differ from `other`.
    fn changed(&self, other: &Components) -> Vec<&'static str> {
        let mut out = Vec::new();
        for (name, a, b) in [
            ("items", &self.items, &other.items),
            ("families", &self.families, &other.families),
            ("jobs", &self.jobs, &other.jobs),
            ("images", &self.images, &other.images),
            ("rules", &self.rules, &other.rules),
            ("labels", &self.labels, &other.labels),
        ] {
            if a != b {
                out.push(name);
            }
        }
        out
    }
}

impl Seal {
    pub fn load(dir: &Path) -> Result<Seal> {
        read_toml(&dir.join(SEAL_FILE))
    }
}

impl Set {
    /// The seal components of the set as it stands. The set must check first: a seal over an
    /// inconsistent set binds nothing.
    pub fn components(&self) -> Result<Components> {
        let id = &self.manifest.set.id;
        self.check()?;
        let mut ids: Vec<&str> = self.manifest.item.iter().map(|i| i.id.as_str()).collect();
        ids.sort_unstable();
        let items = sha256_hex(ids.join("\n").as_bytes());
        let mut assignments: Vec<String> = self
            .manifest
            .item
            .iter()
            .map(|i| format!("{}={}", i.id, i.family))
            .collect();
        assignments.sort();
        let families = sha256_hex(assignments.join("\n").as_bytes());
        let mut jobs_text = String::new();
        let mut ordered: Vec<&(String, Job)> = self.jobs.iter().collect();
        ordered.sort_by(|a, b| a.0.cmp(&b.0));
        for (item, job) in ordered {
            jobs_text.push_str(item);
            jobs_text.push('\n');
            jobs_text.push_str(&serde_json::to_string(job).map_err(|e| sets_err(id, e))?);
            jobs_text.push('\n');
        }
        let jobs = sha256_hex(jobs_text.as_bytes());
        let mut image_lines: Vec<String> = Vec::new();
        for img in self.images()? {
            let bytes = std::fs::read(self.dir.join(&img.path))
                .map_err(|e| sets_err(id, format!("cannot read {}: {e}", img.path.display())))?;
            image_lines.push(format!("{} {}", img.path.display(), sha256_hex(&bytes)));
        }
        image_lines.sort();
        image_lines.dedup();
        let images = sha256_hex(image_lines.join("\n").as_bytes());
        let pin = &self.manifest.set.rules;
        let rules_text = obligations::shipped(&pin.base).unwrap_or_default();
        let rules = sha256_hex(format!("{}@{}\n{rules_text}", pin.base, pin.version).as_bytes());
        let labels = match (&self.manifest.set.labels, &self.manifest.set.labels_sha256) {
            (Some(rel), _) => {
                let bytes = std::fs::read(self.dir.join(rel))
                    .map_err(|e| sets_err(id, format!("cannot read {}: {e}", rel.display())))?;
                sha256_hex(&bytes)
            }
            (None, Some(hash)) => hash.to_ascii_lowercase(),
            (None, None) => return Err(sets_err(id, "no labels file and no labels_sha256")),
        };
        Ok(Components {
            items,
            families,
            jobs,
            images,
            rules,
            labels,
        })
    }
}

/// `dejaview sets seal`: checks the set, then writes `seal.toml`. For a blind set whose labels
/// still sit beside the manifest, `labels_out` names the directory the label file is moved to
/// (owner storage, outside the repositories); the manifest then carries `labels_sha256` instead
/// of `labels`. The attestation is the owner's statement that no judge or tool has seen the items.
pub fn seal(dir: &Path, attestation: &str, labels_out: Option<&Path>) -> Result<Seal> {
    let mut set = Set::load(dir)?;
    let id = set.manifest.set.id.clone();
    if set.manifest.set.split == Split::Blind
        && let Some(rel) = set.manifest.set.labels.clone()
    {
        let out = labels_out.ok_or_else(|| {
            sets_err(
                &id,
                "a blind set's labels must leave the set directory before sealing; name the owner storage directory to move them to",
            )
        })?;
        if inside_repository(out) {
            return Err(sets_err(
                &id,
                format!(
                    "{} is inside a repository; blind labels are owner storage and must leave every repository",
                    out.display()
                ),
            ));
        }
        let from = dir.join(&rel);
        let bytes = std::fs::read(&from)
            .map_err(|e| sets_err(&id, format!("cannot read {}: {e}", from.display())))?;
        let hash = sha256_hex(&bytes);
        let name = rel.file_name().ok_or_else(|| {
            sets_err(
                &id,
                format!("labels path {} has no file name", rel.display()),
            )
        })?;
        let to = out.join(name);
        if to.exists() {
            return Err(sets_err(
                &id,
                format!("{} already exists; refusing to overwrite", to.display()),
            ));
        }
        std::fs::create_dir_all(out)
            .map_err(|e| sets_err(&id, format!("cannot create {}: {e}", out.display())))?;
        std::fs::write(&to, &bytes)
            .map_err(|e| sets_err(&id, format!("cannot write {}: {e}", to.display())))?;
        pin_labels_hash(&dir.join(MANIFEST_FILE), &hash)?;
        std::fs::remove_file(&from)
            .map_err(|e| sets_err(&id, format!("cannot remove {}: {e}", from.display())))?;
        set = Set::load(dir)?;
    }
    let components = set.components()?;
    let seal = Seal {
        seal: SealBody {
            sha256: components.overall(),
            created: utc_now(),
            attestation: attestation.to_string(),
            components,
        },
    };
    let text = toml::to_string_pretty(&seal).map_err(|e| sets_err(&id, e))?;
    std::fs::write(dir.join(SEAL_FILE), text)
        .map_err(|e| sets_err(&id, format!("cannot write {SEAL_FILE}: {e}")))?;
    Ok(seal)
}

/// Whether `out` (or, before it exists, its parent) lies inside a git repository: some ancestor
/// holds a `.git`.
fn inside_repository(out: &Path) -> bool {
    let start = std::fs::canonicalize(out)
        .ok()
        .or_else(|| out.parent().and_then(|p| std::fs::canonicalize(p).ok()));
    let Some(mut p) = start else { return false };
    loop {
        if p.join(".git").exists() {
            return true;
        }
        if !p.pop() {
            return false;
        }
    }
}

/// Replaces the manifest's `labels = "…"` line with `labels_sha256 = "…"`, keeping the rest of
/// the hand-written file as it is.
fn pin_labels_hash(manifest: &Path, hash: &str) -> Result<()> {
    let text = std::fs::read_to_string(manifest)
        .map_err(|e| Error::Sets(format!("cannot read {}: {e}", manifest.display())))?;
    let mut replaced = false;
    let out: Vec<String> = text
        .lines()
        .map(|line| {
            if !replaced
                && (line.trim_start().starts_with("labels ")
                    || line.trim_start().starts_with("labels="))
            {
                replaced = true;
                format!("labels_sha256 = \"{hash}\"")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !replaced {
        return Err(Error::Sets(format!(
            "{}: no `labels = …` line to replace",
            manifest.display()
        )));
    }
    std::fs::write(manifest, out.join("\n") + "\n")
        .map_err(|e| Error::Sets(format!("cannot write {}: {e}", manifest.display())))
}

/// `dejaview sets verify-seal`: the set as it stands reproduces every component of its seal.
pub fn verify_seal(dir: &Path) -> Result<()> {
    let sealed = Seal::load(dir)?;
    let set = Set::load(dir)?;
    let id = &set.manifest.set.id;
    let now = set.components()?;
    let changed = sealed.seal.components.changed(&now);
    if !changed.is_empty() {
        return Err(sets_err(
            id,
            format!(
                "the seal of {} no longer holds; changed: {}",
                sealed.seal.created,
                changed.join(", ")
            ),
        ));
    }
    if sealed.seal.sha256 != now.overall() {
        return Err(sets_err(
            id,
            "the seal's sha256 does not match its own components",
        ));
    }
    Ok(())
}

/// The current time as `YYYY-MM-DDTHH:MM:SSZ`.
pub(crate) fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

// ---- fetch ---------------------------------------------------------------------------------

/// What `fetch` did: images downloaded and verified, and images already in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FetchReport {
    pub fetched: usize,
    pub present: usize,
}

/// The URL of one image of a set. `images_repo` is `github.com/<owner>/<repo>` (served raw at the
/// pinned `images_rev`) or an `https://`/`http://` base; images sit at `<set id>/<path>` under
/// either. A directory is not a URL.
pub fn image_url(
    images_repo: &str,
    images_rev: Option<&str>,
    set_id: &str,
    path: &Path,
) -> Result<String> {
    let rel = format!("{set_id}/{}", path.display());
    if let Some(rest) = images_repo.strip_prefix("github.com/") {
        let rev = images_rev.ok_or_else(|| {
            Error::Sets(format!(
                "images_repo {images_repo:?} needs images_rev, the revision the images are fetched from"
            ))
        })?;
        return Ok(format!(
            "https://raw.githubusercontent.com/{}/{rev}/{rel}",
            rest.trim_end_matches('/')
        ));
    }
    if images_repo.starts_with("https://") || images_repo.starts_with("http://") {
        return Ok(format!("{}/{rel}", images_repo.trim_end_matches('/')));
    }
    Err(Error::Sets(format!(
        "images_repo {images_repo:?} is a directory, not a URL"
    )))
}

/// `dejaview sets fetch`: every image the set's jobs pin is placed beside the manifest, from the
/// set's `images_repo`, and refused unless its SHA-256 matches the pin. An image already present
/// with the right hash is left alone; a present image with the wrong hash is replaced. A refused
/// download leaves no file behind.
pub async fn fetch(dir: &Path) -> Result<FetchReport> {
    let set = Set::load(dir)?;
    let meta = &set.manifest.set;
    let id = &meta.id;
    let remote = image_url(
        &meta.images_repo,
        meta.images_rev.as_deref(),
        id,
        Path::new("x"),
    )
    .is_ok();
    let client = if remote {
        Some(
            dejaview_providers::http::Client::new(std::time::Duration::from_secs(120))
                .map_err(|e| sets_err(id, e))?,
        )
    } else {
        None
    };
    let store = dir.join(&meta.images_repo).join(id);
    let mut report = FetchReport::default();
    for img in set.images()? {
        let target = dir.join(&img.path);
        if let Ok(bytes) = std::fs::read(&target)
            && sha256_hex(&bytes) == img.sha256
        {
            report.present += 1;
            continue;
        }
        let bytes = match &client {
            Some(client) => {
                let url = image_url(&meta.images_repo, meta.images_rev.as_deref(), id, &img.path)?;
                client.get_bytes(&url).await.map_err(|e| {
                    sets_err(
                        id,
                        format!(
                            "item {}: image {}: fetching {url}: {e}",
                            img.item, img.image
                        ),
                    )
                })?
            }
            None => {
                let from = store.join(&img.path);
                std::fs::read(&from).map_err(|e| {
                    sets_err(
                        id,
                        format!(
                            "item {}: image {}: cannot read {}: {e}",
                            img.item,
                            img.image,
                            from.display()
                        ),
                    )
                })?
            }
        };
        let actual = sha256_hex(&bytes);
        if actual != img.sha256 {
            return Err(sets_err(
                id,
                format!(
                    "item {}: image {} ({}) arrived with sha256 {actual}, the job pins {}; nothing written",
                    img.item,
                    img.image,
                    img.path.display(),
                    img.sha256
                ),
            ));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| sets_err(id, format!("cannot create {}: {e}", parent.display())))?;
        }
        let part = target.with_extension(match target.extension().and_then(|e| e.to_str()) {
            Some(ext) => format!("{ext}.part"),
            None => "part".to_string(),
        });
        std::fs::write(&part, &bytes)
            .map_err(|e| sets_err(id, format!("cannot write {}: {e}", part.display())))?;
        std::fs::rename(&part, &target)
            .map_err(|e| sets_err(id, format!("cannot place {}: {e}", target.display())))?;
        report.fetched += 1;
    }
    Ok(report)
}
