//! The tool manifest (SPEC "Manifest"): everything a tool says about itself, rendered once to the
//! provider as the tool definition and once into the prompt as guidance.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const MANIFEST_VERSION: u32 = 1;

fn d_manifest_version() -> u32 {
    MANIFEST_VERSION
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CostTier {
    Cheap,
    Moderate,
    Expensive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    pub tier: CostTier,
    /// Rough duration of one call, in seconds.
    pub seconds: f64,
}

/// Upper bounds on everything a tool may return. The harness refuses an envelope over any bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub max_images: u32,
    pub max_long_edge: u32,
    pub max_image_bytes: u64,
    pub max_summary_bytes: u64,
    pub max_regions: u32,
    pub max_region_label_bytes: u64,
    pub max_measurements: u32,
    pub max_transforms: u32,
    pub max_envelope_bytes: u64,
}

/// Bytes the presentation of one result adds per region, measurement and image beyond their own
/// text: ids, a location at its largest (a 64-vertex polygon with every nullable field written),
/// scores, a frame line with a nine-number mapping, and framing.
const PRESENTED_REGION_BYTES: u64 = 64 * 22 + 160;
const PRESENTED_MEASUREMENT_BYTES: u64 = 160;
const PRESENTED_IMAGE_BYTES: u64 = 320;
const PRESENTED_FRAMING_BYTES: u64 = 120;

impl Bounds {
    /// The most text one result can put before the model: the presentation carries the summary,
    /// the measurements, the regions with their checklist ids and the images' frame lines, never
    /// the image bytes (those travel as images and are priced as such) nor the transforms. The
    /// envelope planner prices this, not `max_envelope_bytes`, which bounds the stored envelope
    /// with its inline pixels.
    pub fn presented_bytes(&self) -> u64 {
        PRESENTED_FRAMING_BYTES
            + self.max_summary_bytes
            + u64::from(self.max_regions) * (self.max_region_label_bytes + PRESENTED_REGION_BYTES)
            + u64::from(self.max_measurements) * PRESENTED_MEASUREMENT_BYTES
            + u64::from(self.max_images) * PRESENTED_IMAGE_BYTES
    }
}

/// A tool's self-description. `model_schema` is what the provider receives; `input_schema` is
/// `model_schema` plus the required `_squint` context; `output_schema` names the measurements
/// the tool may return (the envelope shape itself is fixed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The manifest schema version (`MANIFEST_VERSION`); absent means the current one.
    #[serde(default = "d_manifest_version")]
    pub manifest_version: u32,
    pub name: String,
    pub version: String,
    pub purpose: String,
    pub when_to_use: String,
    pub when_not_to_use: String,
    pub output_guidance: String,
    pub cost: Cost,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub pairs_with: Vec<String>,
    /// Arguments that take image ids.
    #[serde(default)]
    pub image_args: Vec<String>,
    /// Arguments that take frame ids.
    #[serde(default)]
    pub frame_args: Vec<String>,
    #[serde(default)]
    pub accepts_hints: Vec<String>,
    /// What its regions are evidence of; obligations bind to kinds, never to tools.
    #[serde(default)]
    pub evidence_kind: Option<String>,
    pub returns: Bounds,
    #[serde(default)]
    pub frames_out: Vec<String>,
    #[serde(default)]
    pub concurrent: bool,
    #[serde(default)]
    pub preconditions: String,
    /// The evaluation projections this tool supports (`projection::VISUAL_ONLY`); an arm may run
    /// the tool under one of them.
    #[serde(default)]
    pub projections: Vec<String>,
    pub model_schema: serde_json::Value,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
}

/// A manifest file: `[tool]` plus the schema tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    tool: Manifest,
}

impl Manifest {
    /// Parses a TOML manifest (`[tool] …` with inline schema tables) and validates it.
    pub fn from_toml(text: &str) -> Result<Manifest> {
        let file: ManifestFile =
            toml::from_str(text).map_err(|e| Error::Tool(format!("manifest: {e}")))?;
        file.tool.validate()?;
        Ok(file.tool)
    }

    /// Parses a JSON manifest (the shape an MCP server sends under its `squint` annotation).
    pub fn from_json(value: &serde_json::Value) -> Result<Manifest> {
        let m: Manifest = serde_json::from_value(value.clone())
            .map_err(|e| Error::Tool(format!("manifest: {e}")))?;
        m.validate()?;
        Ok(m)
    }

    /// Required text fields non-empty, a sane name, schemas are objects, and the input schema is
    /// the model schema plus a required `_squint`.
    pub fn validate(&self) -> Result<()> {
        if self.manifest_version != MANIFEST_VERSION {
            return Err(Error::Tool(format!(
                "{}: manifest_version must be {MANIFEST_VERSION}, got {}",
                self.name, self.manifest_version
            )));
        }
        let name_ok = !self.name.is_empty()
            && self
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && self
                .name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase());
        if !name_ok {
            return Err(Error::Tool(format!(
                "manifest: name {:?} must be lower snake case",
                self.name
            )));
        }
        for (field, value) in [
            ("version", &self.version),
            ("purpose", &self.purpose),
            ("when_to_use", &self.when_to_use),
            ("when_not_to_use", &self.when_not_to_use),
            ("output_guidance", &self.output_guidance),
        ] {
            if value.trim().is_empty() {
                return Err(Error::Tool(format!(
                    "manifest {}: {field} must not be empty",
                    self.name
                )));
            }
        }
        for p in &self.projections {
            if !super::projection::is_known(p) {
                return Err(Error::Tool(format!(
                    "manifest {}: unknown projection {p:?}",
                    self.name
                )));
            }
            if self.projections.iter().filter(|q| *q == p).count() > 1 {
                return Err(Error::Tool(format!(
                    "manifest {}: projection {p:?} declared twice",
                    self.name
                )));
            }
        }
        if !(self.cost.seconds.is_finite() && self.cost.seconds >= 0.0) {
            return Err(Error::Tool(format!(
                "manifest {}: cost.seconds must be a non-negative number",
                self.name
            )));
        }
        for (field, schema) in [
            ("model_schema", &self.model_schema),
            ("input_schema", &self.input_schema),
            ("output_schema", &self.output_schema),
        ] {
            if schema.get("type") != Some(&serde_json::Value::String("object".into())) {
                return Err(Error::Tool(format!(
                    "manifest {}: {field} must be a JSON Schema object type",
                    self.name
                )));
            }
        }
        let input_required = self
            .input_schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|a| a.iter().any(|v| v.as_str() == Some(super::CONTEXT_KEY)))
            .unwrap_or(false);
        if !input_required {
            return Err(Error::Tool(format!(
                "manifest {}: input_schema must require {:?}",
                self.name,
                super::CONTEXT_KEY
            )));
        }
        if self
            .model_schema
            .get("properties")
            .and_then(|p| p.get(super::CONTEXT_KEY))
            .is_some()
        {
            return Err(Error::Tool(format!(
                "manifest {}: model_schema must not mention {:?}",
                self.name,
                super::CONTEXT_KEY
            )));
        }
        let props = self
            .model_schema
            .get("properties")
            .and_then(|p| p.as_object());
        for arg in self.image_args.iter().chain(&self.frame_args) {
            if !props.is_some_and(|p| p.contains_key(arg)) {
                return Err(Error::Tool(format!(
                    "manifest {}: image/frame argument {arg:?} is not a model_schema property",
                    self.name
                )));
            }
        }
        if self.returns.max_envelope_bytes == 0 {
            return Err(Error::Tool(format!(
                "manifest {}: returns.max_envelope_bytes must be positive",
                self.name
            )));
        }
        Ok(())
    }

    /// The JSON Schema for manifests.
    pub fn json_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(Manifest)).unwrap_or(serde_json::Value::Null)
    }

    /// The tool as the provider receives it.
    pub fn tool_definition(&self) -> crate::provider::ToolDefinition {
        crate::provider::ToolDefinition {
            name: self.name.clone(),
            description: self.purpose.clone(),
            input_schema: self.model_schema.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text() -> String {
        crate::fixture("tools/regions.toml")
    }

    #[test]
    fn the_spec_example_parses_and_validates() {
        let m = Manifest::from_toml(&text()).expect("parses");
        assert_eq!(m.name, "regions");
        assert_eq!(m.evidence_kind.as_deref(), Some("change_region"));
        assert_eq!(m.requires, ["align"]);
        assert_eq!(m.returns.max_regions, 12);
        assert_eq!(m.tool_definition().description, m.purpose);
    }

    #[test]
    fn missing_required_fields_are_refused() {
        let cases = [
            (
                "purpose = \"Finds regions that changed between an aligned pair, with high recall.\"",
                "purpose = \"\"",
                "purpose",
            ),
            ("name = \"regions\"", "name = \"Regions\"", "snake case"),
            ("\"_squint\"]", "]", "must require"),
        ];
        for (from, to, needle) in cases {
            let t = text().replace(from, to);
            assert_ne!(t, text(), "fixture changed for {needle}");
            match Manifest::from_toml(&t) {
                Err(Error::Tool(m)) => assert!(m.contains(needle), "{m}"),
                other => panic!("expected refusal for {needle}, got {other:?}"),
            }
        }
    }

    #[test]
    fn image_args_must_be_schema_properties() {
        let t = text().replace("image_args = [\"image\"]", "image_args = [\"picture\"]");
        assert!(matches!(Manifest::from_toml(&t), Err(Error::Tool(m)) if m.contains("picture")));
    }

    #[test]
    fn the_presented_bound_is_text_the_model_reads_not_the_stored_envelope() {
        let m = Manifest::from_toml(&crate::fixture("tools/regions.toml")).expect("manifest");
        let b = &m.returns;
        let presented = b.presented_bytes();
        assert_eq!(
            presented,
            120 + 2048 + 12 * (120 + 64 * 22 + 160) + 8 * 160 + 320,
            "summary, twelve regions at their largest with labels, eight measurements, one image"
        );
        assert!(
            presented < b.max_envelope_bytes,
            "{presented} vs {}",
            b.max_envelope_bytes
        );
    }
}
