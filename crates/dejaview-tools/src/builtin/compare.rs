//! `edge_similarity` and `change_map`: two frames in one grid compared on structure. Both frames
//! are brought to one working size, their luminance lightly blurred; edges are gradient
//! magnitudes, agreement the normalised cross-correlation of the two edge maps, and the change map
//! the absolute difference of the locally contrast-normalised luminances.

use dejaview_core::Result;
use dejaview_core::frames::Mapping;
use dejaview_core::tool::envelope::Measurement;
use dejaview_core::tool::{DejaviewContext, Envelope, FrameHandle, Manifest, Tool};
use image::RgbImage;

use super::raster;
use crate::align::gray::Gray;

pub const EDGE_MANIFEST: &str = include_str!("../../manifests/edge_similarity.toml");
pub const CHANGE_MAP_MANIFEST: &str = include_str!("../../manifests/change_map.toml");

/// The longer side both frames are brought to before comparing.
const WORKING_SIDE: u32 = 768;
/// A normalised residual beyond this counts as changed, and saturates the map.
const CHANGED: f32 = 1.0;
const SATURATION: f32 = 3.0;

/// The two frames the arguments name, loaded and brought to `frame`'s grid at the working size.
fn pair<'a>(
    args: &'a serde_json::Value,
    ctx: &'a DejaviewContext,
) -> std::result::Result<(&'a str, &'a str, &'a FrameHandle, &'a FrameHandle), String> {
    let id = |key: &str| {
        args.get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{key} must be a frame id"))
    };
    let (frame_id, against_id) = (id("frame")?, id("against")?);
    if frame_id == against_id {
        return Err("frame and against must be two different frames".into());
    }
    let frame = ctx
        .frames
        .get(frame_id)
        .ok_or_else(|| format!("frame: unknown frame id {frame_id:?}"))?;
    let against = ctx
        .frames
        .get(against_id)
        .ok_or_else(|| format!("against: unknown frame id {against_id:?}"))?;
    if frame.projection != against.projection {
        return Err(format!(
            "{frame_id:?} is {:?} and {against_id:?} is {:?}; the frames must share one grid",
            frame.projection, against.projection
        ));
    }
    Ok((frame_id, against_id, frame, against))
}

/// Both images at `frame`'s aspect ratio and the working size, as blurred luminance.
fn working(frame: &RgbImage, against: &RgbImage) -> (Gray, Gray, u32, u32) {
    let (w, h) = frame.dimensions();
    let scale = f64::from(WORKING_SIDE.min(w.max(h))) / f64::from(w.max(h));
    let ww = ((f64::from(w) * scale).round() as u32).max(2);
    let hh = ((f64::from(h) * scale).round() as u32).max(2);
    let fit = |img: &RgbImage| {
        if img.dimensions() == (ww, hh) {
            img.clone()
        } else {
            image::imageops::resize(img, ww, hh, image::imageops::FilterType::Triangle)
        }
    };
    (
        Gray::from_rgb(&fit(frame)).blur3(),
        Gray::from_rgb(&fit(against)).blur3(),
        ww,
        hh,
    )
}

fn edge_magnitude(g: &Gray) -> Gray {
    let (gx, gy) = g.gradients();
    let mut out = Gray::new(g.width, g.height);
    for ((o, a), b) in out.data.iter_mut().zip(gx.data.iter()).zip(gy.data.iter()) {
        *o = (a * a + b * b).sqrt();
    }
    out
}

/// Normalised cross-correlation of two maps; zero when either has no variation.
fn ncc(a: &Gray, b: &Gray) -> f64 {
    let n = a.data.len() as f64;
    let (ma, mb) = (
        a.data.iter().map(|v| f64::from(*v)).sum::<f64>() / n,
        b.data.iter().map(|v| f64::from(*v)).sum::<f64>() / n,
    );
    let (mut cov, mut va, mut vb) = (0.0f64, 0.0f64, 0.0f64);
    for (x, y) in a.data.iter().zip(b.data.iter()) {
        let (dx, dy) = (f64::from(*x) - ma, f64::from(*y) - mb);
        cov += dx * dy;
        va += dx * dx;
        vb += dy * dy;
    }
    if va <= 1e-12 || vb <= 1e-12 {
        return 0.0;
    }
    (cov / (va * vb).sqrt()).clamp(-1.0, 1.0)
}

fn measurement(name: &str, value: f64, unit: &str, frame: &str) -> Measurement {
    Measurement {
        name: name.into(),
        value,
        unit: unit.into(),
        frame: Some(frame.into()),
    }
}

pub struct EdgeSimilarity {
    manifest: Manifest,
}

impl EdgeSimilarity {
    pub fn new() -> EdgeSimilarity {
        EdgeSimilarity {
            manifest: Manifest::from_toml(EDGE_MANIFEST)
                .expect("the edge_similarity manifest is valid"),
        }
    }
}

impl Default for EdgeSimilarity {
    fn default() -> Self {
        EdgeSimilarity::new()
    }
}

impl Tool for EdgeSimilarity {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn call(&self, args: &serde_json::Value, ctx: &DejaviewContext) -> Result<Envelope> {
        let started = std::time::Instant::now();
        let (frame_id, against_id, frame, against) = match pair(args, ctx) {
            Ok(p) => p,
            Err(m) => return Ok(super::invalid_args(&self.manifest, m)),
        };
        let a = raster::load(frame_id, frame)?;
        let b = raster::load(against_id, against)?;
        let (ga, gb, _, _) = working(&a, &b);
        let (ea, eb) = (edge_magnitude(&ga), edge_magnitude(&gb));
        let similarity = ncc(&ea, &eb);
        let energy =
            |e: &Gray| e.data.iter().map(|v| f64::from(*v)).sum::<f64>() / e.data.len() as f64;
        let weaker = energy(&ea).min(energy(&eb));
        let mut envelope = super::with_images(&self.manifest, String::new(), Vec::new(), started);
        envelope.measurements = vec![
            measurement("edge_similarity", similarity, "ratio", frame_id),
            measurement("edge_energy", weaker, "intensity per pixel", frame_id),
        ];
        envelope.summary = format!(
            "Edge similarity of {frame_id} and {against_id}: {similarity:.3} (1 is the same structure; the weaker frame's edge energy is {weaker:.4})."
        );
        Ok(envelope)
    }
}

pub struct ChangeMap {
    manifest: Manifest,
}

impl ChangeMap {
    pub fn new() -> ChangeMap {
        ChangeMap {
            manifest: Manifest::from_toml(CHANGE_MAP_MANIFEST)
                .expect("the change_map manifest is valid"),
        }
    }
}

impl Default for ChangeMap {
    fn default() -> Self {
        ChangeMap::new()
    }
}

impl Tool for ChangeMap {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn call(&self, args: &serde_json::Value, ctx: &DejaviewContext) -> Result<Envelope> {
        let started = std::time::Instant::now();
        let (frame_id, against_id, frame, against) = match pair(args, ctx) {
            Ok(p) => p,
            Err(m) => return Ok(super::invalid_args(&self.manifest, m)),
        };
        let a = raster::load(frame_id, frame)?;
        let b = raster::load(against_id, against)?;
        let (ga, gb, w, h) = working(&a, &b);
        let (la, lb) = (ga.lcn(4, 1e-3), gb.lcn(4, 1e-3));
        let mut map = RgbImage::new(w, h);
        let mut changed = 0usize;
        for (p, (x, y)) in map.pixels_mut().zip(la.data.iter().zip(lb.data.iter())) {
            let d = (x - y).abs();
            if d > CHANGED {
                changed += 1;
            }
            let v = ((d / SATURATION).clamp(0.0, 1.0) * 255.0).round() as u8;
            *p = image::Rgb([v, v, v]);
        }
        let fraction = changed as f64 / f64::from(w * h);
        let image = raster::frame(
            &ctx.call_id,
            0,
            frame_id,
            Mapping::Bbox([0.0, 0.0, 1.0, 1.0]),
            &map,
        )?;
        let mut envelope = super::with_images(&self.manifest, String::new(), vec![image], started);
        envelope.measurements = vec![measurement("changed_fraction", fraction, "ratio", frame_id)];
        envelope.summary = format!(
            "Change map of {frame_id} against {against_id} (crop:{}:0, in {frame_id}'s grid): {:.1}% of pixels differ in structure; bright is different, dark the same.",
            ctx.call_id,
            fraction * 100.0
        );
        Ok(envelope)
    }
}
