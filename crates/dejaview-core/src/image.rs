//! Canonical images: the decoded, identified form of a job image. Task 1.2 adds decode bounds,
//! EXIF orientation, sRGB conversion, projection detection and base64 sources.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::job::{ImageSource, ImageSpec, Projection, Role};
use crate::{Error, Result};

/// A loaded job image. The pixel data is not kept here yet (task 1.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalImage {
    pub id: String,
    pub role: Role,
    pub label: Option<String>,
    pub width: u32,
    pub height: u32,
    /// SHA-256 of the original bytes, lowercase hex.
    pub sha256: String,
    pub projection: Projection,
}

/// Loads one image spec, resolving a relative path against `base`.
pub fn load(spec: &ImageSpec, base: &Path) -> Result<CanonicalImage> {
    let bytes = match &spec.source {
        ImageSource::Path { path } => {
            let full = if path.is_absolute() {
                path.clone()
            } else {
                base.join(path)
            };
            std::fs::read(&full).map_err(|e| {
                Error::Image(format!("{}: cannot read {}: {e}", spec.id, full.display()))
            })?
        }
        ImageSource::Base64 { .. } => {
            return Err(Error::Image(format!(
                "{}: base64 sources are decoded from task 1.2 onward",
                spec.id
            )));
        }
    };
    let decoded = image::load_from_memory(&bytes)
        .map_err(|e| Error::Image(format!("{}: cannot decode: {e}", spec.id)))?;
    let sha256 = hex(&Sha256::digest(&bytes));
    Ok(CanonicalImage {
        id: spec.id.clone(),
        role: spec.role,
        label: spec.label.clone(),
        width: decoded.width(),
        height: decoded.height(),
        sha256,
        projection: spec.projection.unwrap_or_default(),
    })
}

/// Lowercase hex of a digest.
pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn loads_the_fixture_png_with_dimensions_and_hash() {
        let spec = ImageSpec {
            id: "candidate".into(),
            role: Role::Candidate,
            label: None,
            source: ImageSource::Path {
                path: "fixtures/images/candidate-64x48.png".into(),
            },
            projection: None,
        };
        let img = load(&spec, &repo_root()).expect("loads");
        assert_eq!((img.width, img.height), (64, 48));
        assert_eq!(img.sha256.len(), 64);
        assert_eq!(img.projection, Projection::Flat);
    }

    #[test]
    fn a_missing_file_is_an_image_error() {
        let spec = ImageSpec {
            id: "x".into(),
            role: Role::Candidate,
            label: None,
            source: ImageSource::Path {
                path: "fixtures/images/does-not-exist.png".into(),
            },
            projection: None,
        };
        assert!(matches!(load(&spec, &repo_root()), Err(Error::Image(_))));
    }
}
