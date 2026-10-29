//! `view`: a crop of a flat (or equirectangular) frame by normalised box, or a square perspective
//! view of an equirectangular frame by yaw, pitch and field of view; the same region in a second
//! frame when asked.

use dejaview_core::Result;
use dejaview_core::frames::Mapping;
use dejaview_core::frames::sphere;
use dejaview_core::job::Projection;
use dejaview_core::tool::envelope::EnvelopeImage;
use dejaview_core::tool::{DejaviewContext, Envelope, Manifest, Tool};
use image::RgbImage;

use super::raster;

pub const MANIFEST: &str = include_str!("../../manifests/view.toml");

/// The longer side of a crop is capped here; a perspective view is square at this size at most.
const MAX_SIDE: u32 = 1024;
/// A perspective view is never smaller than this.
const MIN_VIEW: u32 = 256;
/// A crop smaller than this on either side is refused as too small to show anything.
const MIN_CROP: u32 = 8;

pub struct View {
    manifest: Manifest,
}

impl View {
    pub fn new() -> View {
        View {
            manifest: Manifest::from_toml(MANIFEST).expect("the view manifest is valid"),
        }
    }
}

impl Default for View {
    fn default() -> Self {
        View::new()
    }
}

enum Request {
    Crop([f64; 4]),
    Perspective(sphere::View),
}

impl Tool for View {
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
        let also_id = match args.get("also") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(s)) => Some(s.as_str()),
            Some(_) => return invalid("also must be a frame id".into()),
        };
        let also = match also_id {
            None => None,
            Some(id) if id == frame_id => {
                return invalid("also must name a different frame from frame".into());
            }
            Some(id) => match ctx.frames.get(id) {
                Some(h) if h.projection == handle.projection => Some((id, h)),
                Some(_) => {
                    return invalid(format!(
                        "also: frame {id:?} has a different projection from {frame_id:?}"
                    ));
                }
                None => return invalid(format!("also: unknown frame id {id:?}")),
            },
        };
        let request = match parse_request(args, handle.projection) {
            Ok(r) => r,
            Err(m) => return invalid(m),
        };
        let mut images = Vec::new();
        let mut summary = String::new();
        for (n, (id, h)) in std::iter::once((frame_id, handle)).chain(also).enumerate() {
            let pixels = raster::load(id, h)?;
            let (img, mapping, text) = match &request {
                Request::Crop(b) => match crop(&pixels, b) {
                    Ok((img, mapping, text)) => (img, mapping, text),
                    Err(m) => return invalid(m),
                },
                Request::Perspective(v) => perspective(&pixels, v),
            };
            images.push(raster::frame(&ctx.call_id, n, id, mapping, &img)?);
            if n == 0 {
                summary = format!("{text} of {id}");
            } else {
                summary.push_str(&format!(", and the same of {id}"));
            }
        }
        summary.push_str(&format!(
            " ({}).",
            images
                .iter()
                .map(|i: &EnvelopeImage| format!("{} {}×{}", i.frame, i.width, i.height))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        Ok(super::with_images(&self.manifest, summary, images, started))
    }
}

/// Which of the two forms the arguments ask for, checked against the frame's projection.
fn parse_request(
    args: &serde_json::Value,
    projection: Projection,
) -> std::result::Result<Request, String> {
    let number = |key: &str| -> std::result::Result<Option<f64>, String> {
        match args.get(key) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(v) => v
                .as_f64()
                .filter(|f| f.is_finite())
                .map(Some)
                .ok_or_else(|| format!("{key} must be a number")),
        }
    };
    let bbox = match args.get("bbox") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Array(a)) if a.len() == 4 => {
            let mut b = [0.0; 4];
            for (slot, v) in b.iter_mut().zip(a.iter()) {
                *slot = v
                    .as_f64()
                    .filter(|f| f.is_finite())
                    .ok_or_else(|| "bbox entries must be numbers".to_string())?;
            }
            Some(b)
        }
        Some(_) => return Err("bbox must be [left, top, right, bottom]".into()),
    };
    let (yaw, pitch, fov) = (number("yaw")?, number("pitch")?, number("fov")?);
    let angles = [yaw, pitch, fov].iter().filter(|v| v.is_some()).count();
    match (bbox, angles) {
        (Some(_), n) if n > 0 => Err("give either bbox or yaw, pitch and fov, not both".into()),
        (Some(b), _) => {
            if b.iter().any(|v| !(0.0..=1.0).contains(v)) || b[0] >= b[2] || b[1] >= b[3] {
                return Err(format!(
                    "bbox {b:?} must be fractions with left < right and top < bottom"
                ));
            }
            Ok(Request::Crop(b))
        }
        (None, 0) => {
            Err("give bbox for a crop, or yaw, pitch and fov for a perspective view".into())
        }
        (None, _) if projection != Projection::Equirectangular => Err(FLAT_FRAME.into()),
        (None, 3) => {
            let (yaw, pitch, fov) = (yaw.unwrap_or(0.0), pitch.unwrap_or(0.0), fov.unwrap_or(0.0));
            if !(-180.0..180.0).contains(&yaw) {
                return Err(format!("yaw {yaw} outside [-180, 180)"));
            }
            if !(-90.0..=90.0).contains(&pitch) {
                return Err(format!("pitch {pitch} outside [-90, 90]"));
            }
            if !(10.0..=120.0).contains(&fov) {
                return Err(format!("fov {fov} outside [10, 120]"));
            }
            Ok(Request::Perspective(sphere::View { yaw, pitch, fov }))
        }
        (None, _) => {
            Err("yaw, pitch and fov go together: send all three for a perspective view".into())
        }
    }
}

/// The refusal for angles on a flat frame: it says what to send instead, because a model told
/// only that the angles go together sends the same call again.
const FLAT_FRAME: &str = "this frame is flat: send bbox [left, top, right, bottom] as fractions of it; yaw, pitch and fov are for equirectangular panoramas";

/// A pixel-aligned crop: the box snaps outward to whole pixels, the mapping records the box
/// actually cut, and the result is scaled down to the long-edge cap when larger.
fn crop(img: &RgbImage, b: &[f64; 4]) -> std::result::Result<(RgbImage, Mapping, String), String> {
    let (w, h) = (img.width(), img.height());
    let x0 = ((b[0] * f64::from(w)).floor() as u32).min(w - 1);
    let y0 = ((b[1] * f64::from(h)).floor() as u32).min(h - 1);
    let x1 = ((b[2] * f64::from(w)).ceil() as u32).clamp(x0 + 1, w);
    let y1 = ((b[3] * f64::from(h)).ceil() as u32).clamp(y0 + 1, h);
    if x1 - x0 < MIN_CROP || y1 - y0 < MIN_CROP {
        return Err(format!(
            "bbox {b:?} covers only {}×{} pixels; at least {MIN_CROP}×{MIN_CROP} are needed",
            x1 - x0,
            y1 - y0
        ));
    }
    let cut = image::imageops::crop_imm(img, x0, y0, x1 - x0, y1 - y0).to_image();
    let out = raster::fit(cut, MAX_SIDE);
    let mapping = Mapping::Bbox([
        f64::from(x0) / f64::from(w),
        f64::from(y0) / f64::from(h),
        f64::from(x1) / f64::from(w),
        f64::from(y1) / f64::from(h),
    ]);
    Ok((
        out,
        mapping,
        format!(
            "Crop [{:.3}, {:.3}, {:.3}, {:.3}]",
            f64::from(x0) / f64::from(w),
            f64::from(y0) / f64::from(h),
            f64::from(x1) / f64::from(w),
            f64::from(y1) / f64::from(h)
        ),
    ))
}

/// The side of a perspective view: about the panorama's own resolution over the field of view,
/// within the caps.
pub fn view_side(pano_width: u32, fov: f64) -> u32 {
    ((f64::from(pano_width) * fov / 360.0).round() as u32).clamp(MIN_VIEW, MAX_SIDE)
}

/// A square perspective view rendered by inverse mapping every pixel to the sphere and sampling
/// the panorama bilinearly, wrapping across the seam.
fn perspective(img: &RgbImage, v: &sphere::View) -> (RgbImage, Mapping, String) {
    let side = view_side(img.width(), v.fov);
    let (pw, ph) = (f64::from(img.width()), f64::from(img.height()));
    let mut out = RgbImage::new(side, side);
    for (u, w, p) in out.enumerate_pixels_mut() {
        let nx = (f64::from(u) + 0.5) / f64::from(side);
        let ny = (f64::from(w) + 0.5) / f64::from(side);
        let e = sphere::equirect_of(v.direction(nx, ny));
        *p = image::Rgb(raster::sample(img, e[0] * pw, e[1] * ph, true));
    }
    (
        out,
        Mapping::View {
            yaw: v.yaw,
            pitch: v.pitch,
            fov: v.fov,
        },
        format!(
            "Perspective view at yaw {}°, pitch {}°, fov {}°",
            v.yaw, v.pitch, v.fov
        ),
    )
}
