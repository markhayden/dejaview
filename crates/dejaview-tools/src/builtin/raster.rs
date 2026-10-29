//! Pixel work shared by the built-ins: loading a frame's lossless pixels from the handle the
//! harness injected, bilinear sampling, scaling, and encoding a result as a PNG frame.

use dejaview_core::frames::Mapping;
use dejaview_core::tool::FrameHandle;
use dejaview_core::tool::envelope::EnvelopeImage;
use dejaview_core::{Error, Result};
use image::RgbImage;

/// Loads a frame's pixels from the path the harness injected (never a path from the model).
pub fn load(frame: &str, handle: &FrameHandle) -> Result<RgbImage> {
    let path = handle
        .pixels
        .path
        .as_deref()
        .ok_or_else(|| Error::Tool(format!("frame {frame:?} has no pixel path")))?;
    let img = image::open(path)
        .map_err(|e| Error::Tool(format!("frame {frame:?}: cannot decode its pixels: {e}")))?;
    Ok(img.to_rgb8())
}

/// Bilinear sample at a continuous edge-based coordinate (pixel `i` covers `[i, i+1)`), with
/// horizontal wrap-around when `wrap` is set and clamping otherwise.
pub fn sample(img: &RgbImage, x: f64, y: f64, wrap: bool) -> [u8; 3] {
    let (w, h) = (img.width() as f64, img.height() as f64);
    let fx = if wrap {
        (x - 0.5).rem_euclid(w)
    } else {
        (x - 0.5).clamp(0.0, w - 1.0)
    };
    let fy = (y - 0.5).clamp(0.0, h - 1.0);
    let x0 = (fx.floor() as u32).min(img.width() - 1);
    let y0 = (fy.floor() as u32).min(img.height() - 1);
    let x1 = if wrap {
        (x0 + 1) % img.width()
    } else {
        (x0 + 1).min(img.width() - 1)
    };
    let y1 = (y0 + 1).min(img.height() - 1);
    let (ax, ay) = (fx - fx.floor(), fy - fy.floor());
    let p = |x: u32, y: u32| img.get_pixel(x, y).0;
    let (a, b, c, d) = (p(x0, y0), p(x1, y0), p(x0, y1), p(x1, y1));
    let mut out = [0u8; 3];
    for (k, o) in out.iter_mut().enumerate() {
        let top = f64::from(a[k]) * (1.0 - ax) + f64::from(b[k]) * ax;
        let bottom = f64::from(c[k]) * (1.0 - ax) + f64::from(d[k]) * ax;
        *o = (top * (1.0 - ay) + bottom * ay).round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Scales an image down so its longer side is `long_edge` (a triangle filter); returns it
/// unchanged when it is already no larger.
pub fn fit(img: RgbImage, long_edge: u32) -> RgbImage {
    let (w, h) = img.dimensions();
    let longer = w.max(h);
    if longer <= long_edge {
        return img;
    }
    let scale = f64::from(long_edge) / f64::from(longer);
    let nw = ((f64::from(w) * scale).round() as u32).max(1);
    let nh = ((f64::from(h) * scale).round() as u32).max(1);
    image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
}

/// Encodes a result as the `n`th crop frame of a call: lossless PNG, inline.
pub fn frame(
    call_id: &str,
    n: usize,
    parent: &str,
    mapping: Mapping,
    img: &RgbImage,
) -> Result<EnvelopeImage> {
    let mut bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .map_err(|e| Error::Tool(format!("cannot encode the result as PNG: {e}")))?;
    Ok(EnvelopeImage {
        frame: format!("crop:{call_id}:{n}"),
        parent: parent.to_string(),
        mapping,
        media_type: "image/png".into(),
        width: img.width(),
        height: img.height(),
        base64: Some(dejaview_core::image::base64_encode(&bytes)),
    })
}
