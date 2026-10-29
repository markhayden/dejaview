//! `resize`: a frame scaled down to a chosen long edge; the mapping covers the whole parent.

use dejaview_core::Result;
use dejaview_core::frames::Mapping;
use dejaview_core::tool::{DejaviewContext, Envelope, Manifest, Tool};

use super::raster;

pub const MANIFEST: &str = include_str!("../../manifests/resize.toml");

pub struct Resize {
    manifest: Manifest,
}

impl Resize {
    pub fn new() -> Resize {
        Resize {
            manifest: Manifest::from_toml(MANIFEST).expect("the resize manifest is valid"),
        }
    }
}

impl Default for Resize {
    fn default() -> Self {
        Resize::new()
    }
}

impl Tool for Resize {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn call(&self, args: &serde_json::Value, ctx: &DejaviewContext) -> Result<Envelope> {
        let started = std::time::Instant::now();
        let invalid = |m: String| Ok(super::invalid_args(&self.manifest, m));
        let Some(frame_id) = args.get("frame").and_then(|v| v.as_str()) else {
            return invalid("frame must be a frame id".into());
        };
        let Some(handle) = ctx.frames.get(frame_id) else {
            return invalid(format!("frame: unknown frame id {frame_id:?}"));
        };
        let Some(long_edge) = args.get("long_edge").and_then(|v| v.as_u64()) else {
            return invalid("long_edge must be a whole number of pixels".into());
        };
        if !(64..=1024).contains(&long_edge) {
            return invalid(format!("long_edge {long_edge} outside [64, 1024]"));
        }
        let longer = u64::from(handle.pixels.width.max(handle.pixels.height));
        if long_edge >= longer {
            return invalid(format!(
                "long_edge {long_edge} would not shrink {frame_id:?}, whose longer side is {longer}; resize never enlarges"
            ));
        }
        let pixels = raster::load(frame_id, handle)?;
        let out = raster::fit(pixels, long_edge as u32);
        let summary = format!(
            "{frame_id} scaled to {}×{} (crop:{}:0).",
            out.width(),
            out.height(),
            ctx.call_id
        );
        let image = raster::frame(
            &ctx.call_id,
            0,
            frame_id,
            Mapping::Bbox([0.0, 0.0, 1.0, 1.0]),
            &out,
        )?;
        Ok(super::with_images(
            &self.manifest,
            summary,
            vec![image],
            started,
        ))
    }
}
