//! The result envelope every tool returns (SPEC "Manifest", result envelope), and the bounds check
//! the harness applies against the manifest.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::manifest::Bounds;
use crate::frames::Location;
use crate::{Error, Result};

pub const ENVELOPE_VERSION: u32 = 1;

fn d_envelope_version() -> u32 {
    ENVELOPE_VERSION
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    MeasuredNothing,
    NotApplicable,
    InvalidArgs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub name: String,
    pub value: f64,
    pub unit: String,
    #[serde(default)]
    pub frame: Option<String>,
}

/// A region a tool points at. Becomes an evidence item on the checklist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub id: String,
    pub location: Location,
    pub label: String,
    pub score: f64,
}

/// An image a tool returned: a new `crop` frame with its parent and mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeImage {
    pub frame: String,
    pub parent: String,
    pub mapping: crate::frames::Mapping,
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    /// The bytes, inline for built-ins and MCP content blocks alike; stored as an artifact later.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub params: Vec<f64>,
    #[serde(default)]
    pub quality: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub tool: String,
    pub version: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub lock_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    /// The envelope schema version (`ENVELOPE_VERSION`); absent means the current one.
    #[serde(default = "d_envelope_version")]
    pub envelope_version: u32,
    pub status: Status,
    pub summary: String,
    #[serde(default)]
    pub measurements: Vec<Measurement>,
    #[serde(default)]
    pub regions: Vec<Region>,
    #[serde(default)]
    pub images: Vec<EnvelopeImage>,
    #[serde(default)]
    pub transforms: Vec<Transform>,
    pub provenance: Provenance,
}

impl Envelope {
    /// Checks every bound in the manifest. A breach is `error: tool`.
    pub fn check_bounds(&self, bounds: &Bounds) -> Result<()> {
        let tool = &self.provenance.tool;
        if self.envelope_version != ENVELOPE_VERSION {
            return Err(Error::Tool(format!(
                "{tool}: envelope_version must be {ENVELOPE_VERSION}, got {}",
                self.envelope_version
            )));
        }
        let over = |what: &str, got: u64, max: u64| -> Result<()> {
            if got > max {
                Err(Error::Tool(format!(
                    "{tool}: envelope {what} {got} exceeds the manifest bound {max}"
                )))
            } else {
                Ok(())
            }
        };
        over(
            "summary bytes",
            self.summary.len() as u64,
            bounds.max_summary_bytes,
        )?;
        over(
            "regions",
            self.regions.len() as u64,
            u64::from(bounds.max_regions),
        )?;
        for r in &self.regions {
            over(
                "region label bytes",
                r.label.len() as u64,
                bounds.max_region_label_bytes,
            )?;
        }
        over(
            "measurements",
            self.measurements.len() as u64,
            u64::from(bounds.max_measurements),
        )?;
        over(
            "transforms",
            self.transforms.len() as u64,
            u64::from(bounds.max_transforms),
        )?;
        over(
            "images",
            self.images.len() as u64,
            u64::from(bounds.max_images),
        )?;
        for img in &self.images {
            over(
                "image long edge",
                u64::from(img.width.max(img.height)),
                u64::from(bounds.max_long_edge),
            )?;
            if let Some(b64) = &img.base64 {
                // Decoded size is three quarters of the base64 length, rounded.
                over(
                    "image bytes",
                    (b64.len() as u64 * 3) / 4,
                    bounds.max_image_bytes,
                )?;
            }
        }
        let serialised = serde_json::to_vec(self)?.len() as u64;
        over("serialised bytes", serialised, bounds.max_envelope_bytes)
    }

    /// The JSON Schema for envelopes.
    pub fn json_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(Envelope)).unwrap_or(serde_json::Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::LocationKind;

    fn bounds() -> Bounds {
        Bounds {
            max_images: 1,
            max_long_edge: 1024,
            max_image_bytes: 2_000_000,
            max_summary_bytes: 64,
            max_regions: 2,
            max_region_label_bytes: 16,
            max_measurements: 1,
            max_transforms: 0,
            max_envelope_bytes: 4096,
        }
    }

    fn region(label: &str) -> Region {
        Region {
            id: "r1".into(),
            location: Location {
                frame: "raw:candidate".into(),
                kind: LocationKind::Bbox,
                bbox: Some([0.1, 0.1, 0.2, 0.2]),
                polygon: None,
                yaw: None,
                pitch: None,
                fov: None,
            },
            label: label.into(),
            score: 0.9,
        }
    }

    fn envelope() -> Envelope {
        Envelope {
            envelope_version: 1,
            status: Status::Ok,
            summary: "two regions".into(),
            measurements: vec![],
            regions: vec![region("floor"), region("wall")],
            images: vec![],
            transforms: vec![],
            provenance: Provenance {
                tool: "fake".into(),
                version: "1".into(),
                duration_ms: 1,
                lock_digest: None,
            },
        }
    }

    #[test]
    fn within_bounds_passes() {
        envelope().check_bounds(&bounds()).expect("ok");
    }

    #[test]
    fn each_bound_is_enforced() {
        let mut e = envelope();
        e.regions.push(region("ceiling"));
        assert!(
            matches!(e.check_bounds(&bounds()), Err(Error::Tool(m)) if m.contains("regions 3"))
        );
        let mut e = envelope();
        e.summary = "x".repeat(65);
        assert!(matches!(e.check_bounds(&bounds()), Err(Error::Tool(m)) if m.contains("summary")));
        let mut e = envelope();
        e.regions[0].label = "a very long label indeed".into();
        assert!(matches!(e.check_bounds(&bounds()), Err(Error::Tool(m)) if m.contains("label")));
        let mut e = envelope();
        e.transforms.push(Transform {
            id: "t".into(),
            from: "a".into(),
            to: "b".into(),
            kind: "h".into(),
            params: vec![],
            quality: serde_json::Value::Null,
        });
        assert!(
            matches!(e.check_bounds(&bounds()), Err(Error::Tool(m)) if m.contains("transforms"))
        );
        let mut e = envelope();
        e.images.push(EnvelopeImage {
            frame: "crop:call:1:0".into(),
            parent: "raw:candidate".into(),
            mapping: crate::frames::Mapping::Bbox([0.0, 0.0, 0.5, 0.5]),
            media_type: "image/png".into(),
            width: 2048,
            height: 10,
            base64: None,
        });
        assert!(
            matches!(e.check_bounds(&bounds()), Err(Error::Tool(m)) if m.contains("long edge"))
        );
    }
}
