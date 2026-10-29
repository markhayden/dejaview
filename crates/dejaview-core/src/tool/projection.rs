//! Evaluation projections (SPEC "Evaluation projections"): a named reduction of a tool's envelope
//! that an evaluation arm runs the tool under, applied by the harness before anything reaches the
//! model or the checklist. A tool declares the projections it supports in its manifest.

use std::sync::Arc;

use super::{DejaviewContext, Envelope, Manifest, Status, Tool};
use crate::{Error, Result};

/// Keeps the visual artifacts (`images`, and `transforms` with their `params`) and withholds the
/// diagnostics: `quality` removed, `measurements` and `regions` emptied, the summary replaced by
/// a fixed sentence, `status` mapped to `ok` or `not_applicable` (`invalid_args` stays, since it
/// answers the model's own mistake). Arms that use it see identical pictures and no evidence.
pub const VISUAL_ONLY: &str = "visual_only";

/// The summary a `visual_only` result carries when the tool computed something.
pub const VISUAL_ONLY_COMPUTED: &str = "Result computed; its measurements are withheld.";
/// The summary a `visual_only` result carries when the tool had nothing to give.
pub const VISUAL_ONLY_UNAVAILABLE: &str = "Result not available.";

/// Whether `name` is a projection the harness knows.
pub fn is_known(name: &str) -> bool {
    name == VISUAL_ONLY
}

/// Applies the named projection to an envelope.
pub fn apply(name: &str, mut envelope: Envelope) -> Result<Envelope> {
    match name {
        VISUAL_ONLY => {
            if envelope.status == Status::InvalidArgs {
                return Ok(envelope);
            }
            envelope.measurements.clear();
            envelope.regions.clear();
            for t in &mut envelope.transforms {
                t.quality = serde_json::Value::Null;
            }
            let computed = envelope.status == Status::Ok
                && !(envelope.images.is_empty() && envelope.transforms.is_empty());
            envelope.status = if computed {
                Status::Ok
            } else {
                Status::NotApplicable
            };
            envelope.summary = if computed {
                VISUAL_ONLY_COMPUTED.to_string()
            } else {
                VISUAL_ONLY_UNAVAILABLE.to_string()
            };
            Ok(envelope)
        }
        other => Err(Error::Config(format!("unknown projection {other:?}"))),
    }
}

/// A tool run under a projection: the same manifest, every envelope projected before it returns.
pub struct Projected {
    inner: Arc<dyn Tool>,
    projection: String,
}

impl Projected {
    /// Wraps `tool`; the tool must declare the projection and the harness must know it.
    pub fn new(inner: Arc<dyn Tool>, projection: &str) -> Result<Projected> {
        let m = inner.manifest();
        if !is_known(projection) {
            return Err(Error::Config(format!(
                "tool {:?}: unknown projection {projection:?}",
                m.name
            )));
        }
        if !m.projections.iter().any(|p| p == projection) {
            return Err(Error::Config(format!(
                "tool {:?} does not declare the projection {projection:?}",
                m.name
            )));
        }
        Ok(Projected {
            inner,
            projection: projection.to_string(),
        })
    }

    pub fn projection(&self) -> &str {
        &self.projection
    }
}

impl Tool for Projected {
    fn manifest(&self) -> &Manifest {
        self.inner.manifest()
    }

    fn call(&self, args: &serde_json::Value, ctx: &DejaviewContext) -> Result<Envelope> {
        apply(&self.projection, self.inner.call(args, ctx)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::envelope::{Measurement, Provenance, Region, Transform};

    fn envelope(status: Status) -> Envelope {
        Envelope {
            envelope_version: 1,
            status,
            summary: "found 2 regions".into(),
            measurements: vec![Measurement {
                name: "edge_similarity".into(),
                value: 0.8,
                unit: "ratio".into(),
                frame: None,
            }],
            regions: vec![Region {
                id: "r1".into(),
                location: crate::frames::Location {
                    frame: "raw:candidate".into(),
                    kind: crate::frames::LocationKind::WholeImage,
                    bbox: None,
                    polygon: None,
                    yaw: None,
                    pitch: None,
                    fov: None,
                },
                label: "x".into(),
                score: 0.9,
            }],
            images: Vec::new(),
            transforms: vec![Transform {
                id: "t1".into(),
                from: "raw:candidate".into(),
                to: "raw:source".into(),
                kind: "homography".into(),
                params: vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
                quality: serde_json::json!({"inlier_fraction": 0.9}),
            }],
            provenance: Provenance {
                tool: "align".into(),
                version: "1".into(),
                duration_ms: 1,
                lock_digest: None,
            },
        }
    }

    #[test]
    fn visual_only_keeps_the_pictures_and_withholds_the_diagnostics() {
        let e = apply(VISUAL_ONLY, envelope(Status::Ok)).unwrap();
        assert_eq!(e.status, Status::Ok);
        assert_eq!(e.summary, VISUAL_ONLY_COMPUTED);
        assert!(e.measurements.is_empty() && e.regions.is_empty());
        assert_eq!(e.transforms.len(), 1);
        assert_eq!(e.transforms[0].params.len(), 9);
        assert!(e.transforms[0].quality.is_null());
    }

    #[test]
    fn visual_only_without_a_picture_is_not_applicable() {
        let mut none = envelope(Status::Ok);
        none.transforms.clear();
        let e = apply(VISUAL_ONLY, none).unwrap();
        assert_eq!(e.status, Status::NotApplicable);
        assert_eq!(e.summary, VISUAL_ONLY_UNAVAILABLE);
        let e = apply(VISUAL_ONLY, envelope(Status::MeasuredNothing)).unwrap();
        assert_eq!(e.status, Status::NotApplicable);
        let e = apply(VISUAL_ONLY, envelope(Status::InvalidArgs)).unwrap();
        assert_eq!(
            e.status,
            Status::InvalidArgs,
            "the model's mistake is still answered"
        );
        assert_eq!(e.summary, "found 2 regions");
    }

    #[test]
    fn an_unknown_projection_is_refused() {
        assert!(matches!(
            apply("blurry", envelope(Status::Ok)),
            Err(Error::Config(_))
        ));
    }
}
