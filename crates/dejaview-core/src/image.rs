//! Canonical images: the decoded, identified form of a job image. Decode bounds are checked from
//! the header before pixels are decoded; EXIF orientation is applied; pixels are kept as 8-bit RGB
//! (an embedded ICC profile is recorded but not applied: the `image` crate has no colour
//! management, so "sRGB" here means 8-bit RGB as decoded); the original bytes and the canonical
//! pixels are both hashed; projection comes from the spec, else GPano metadata, else `flat`.

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use image::{DynamicImage, ImageDecoder, ImageReader, metadata::Orientation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::job::{ImageSource, ImageSpec, Projection, Role};
use crate::{Error, Result};

/// Limits applied before decoding. Config `[images]` overrides them (task 1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodeBounds {
    pub max_pixels: u64,
    pub max_bytes: u64,
}

impl Default for DecodeBounds {
    fn default() -> Self {
        DecodeBounds {
            max_pixels: 50_000_000,
            max_bytes: 64 * 1024 * 1024,
        }
    }
}

/// A loaded job image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalImage {
    pub id: String,
    pub role: Role,
    pub label: Option<String>,
    /// Dimensions after orientation normalisation.
    pub width: u32,
    pub height: u32,
    /// SHA-256 of the original bytes, lowercase hex.
    pub sha256: String,
    /// SHA-256 of the canonical pixels (8-bit RGB, row-major, after orientation), lowercase hex.
    pub canonical_sha256: String,
    pub projection: Projection,
    /// Whether the file carried an ICC profile (recorded, not applied).
    pub has_icc_profile: bool,
    /// Non-fatal notes, for example the 2:1 aspect ratio treated as flat.
    pub warnings: Vec<String>,
    /// The canonical pixels, shared with tools; not serialised.
    #[serde(skip)]
    pub pixels: Option<Arc<image::RgbImage>>,
}

/// Loads one image spec, resolving a relative path against `base`.
pub fn load(spec: &ImageSpec, base: &Path, bounds: &DecodeBounds) -> Result<CanonicalImage> {
    let bytes = read_bytes(spec, base)?;
    if bytes.len() as u64 > bounds.max_bytes {
        return Err(Error::Image(format!(
            "{}: {} bytes exceeds the decode bound of {} bytes",
            spec.id,
            bytes.len(),
            bounds.max_bytes
        )));
    }
    let reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|e| Error::Image(format!("{}: cannot read: {e}", spec.id)))?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| Error::Image(format!("{}: cannot decode header: {e}", spec.id)))?;
    let (w, h) = decoder.dimensions();
    let pixels = u64::from(w) * u64::from(h);
    if pixels > bounds.max_pixels {
        return Err(Error::Image(format!(
            "{}: {w}x{h} = {pixels} pixels exceeds the decode bound of {} pixels",
            spec.id, bounds.max_pixels
        )));
    }
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let has_icc_profile = decoder.icc_profile().ok().flatten().is_some();
    let mut dynamic = DynamicImage::from_decoder(decoder)
        .map_err(|e| Error::Image(format!("{}: cannot decode: {e}", spec.id)))?;
    dynamic.apply_orientation(orientation);
    let rgb = dynamic.to_rgb8();

    let mut warnings = Vec::new();
    let projection = match spec.projection {
        Some(p) => p,
        None if has_gpano_equirectangular(&bytes) => Projection::Equirectangular,
        None => {
            if rgb.width() == 2 * rgb.height() {
                warnings.push(format!(
                    "{}: 2:1 aspect ratio but no projection metadata; treated as flat. Set \
                     projection = \"equirectangular\" if it is a panorama.",
                    spec.id
                ));
            }
            Projection::Flat
        }
    };

    Ok(CanonicalImage {
        id: spec.id.clone(),
        role: spec.role,
        label: spec.label.clone(),
        width: rgb.width(),
        height: rgb.height(),
        sha256: hex(&Sha256::digest(&bytes)),
        canonical_sha256: hex(&Sha256::digest(rgb.as_raw())),
        projection,
        has_icc_profile,
        warnings,
        pixels: Some(Arc::new(rgb)),
    })
}

fn read_bytes(spec: &ImageSpec, base: &Path) -> Result<Vec<u8>> {
    match &spec.source {
        ImageSource::Path { path } => {
            let full = if path.is_absolute() {
                path.clone()
            } else {
                base.join(path)
            };
            std::fs::read(&full).map_err(|e| {
                Error::Image(format!("{}: cannot read {}: {e}", spec.id, full.display()))
            })
        }
        ImageSource::Base64 { base64, .. } => base64_decode(base64)
            .map_err(|e| Error::Image(format!("{}: invalid base64: {e}", spec.id))),
    }
}

/// True when the bytes carry XMP GPano metadata declaring an equirectangular projection. A plain
/// byte scan, format-agnostic, which is enough for the two forms XMP uses (attribute and element).
fn has_gpano_equirectangular(bytes: &[u8]) -> bool {
    const KEY: &[u8] = b"GPano:ProjectionType";
    const VALUE: &[u8] = b"equirectangular";
    let mut from = 0;
    while let Some(pos) = find(&bytes[from..], KEY) {
        let start = from + pos + KEY.len();
        let end = (start + 64).min(bytes.len());
        if find(&bytes[start..end], VALUE).is_some() {
            return true;
        }
        from = start;
    }
    false
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Lowercase hex of a digest.
pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding.
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Standard base64 decoding; whitespace is ignored, padding is optional, anything else fails.
pub fn base64_decode(text: &str) -> std::result::Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for (i, c) in text.bytes().enumerate() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            other => return Err(format!("unexpected byte {other:#04x} at position {i}")),
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_every_remainder() {
        for len in 0..10usize {
            let bytes: Vec<u8> = (0..len as u8).map(|i| i.wrapping_mul(37)).collect();
            let text = base64_encode(&bytes);
            assert_eq!(
                base64_decode(&text).expect("decodes"),
                bytes,
                "len {len}: {text}"
            );
        }
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(
            base64_decode("TW Fu\n").expect("whitespace ignored"),
            b"Man"
        );
        assert!(base64_decode("TW*u").is_err());
    }

    #[test]
    fn gpano_scan_finds_attribute_and_element_forms() {
        assert!(has_gpano_equirectangular(
            br#"<rdf GPano:ProjectionType="equirectangular"/>"#
        ));
        assert!(has_gpano_equirectangular(
            b"<GPano:ProjectionType>equirectangular</GPano:ProjectionType>"
        ));
        assert!(!has_gpano_equirectangular(
            b"<GPano:ProjectionType>cylindrical</GPano:ProjectionType>"
        ));
        assert!(!has_gpano_equirectangular(b"no metadata here"));
    }

    #[test]
    fn hex_is_lowercase_and_zero_padded() {
        assert_eq!(hex(&[0, 1, 255, 16]), "0001ff10");
    }
}
