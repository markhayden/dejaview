//! The built-in tools: deterministic, in-process, pure functions over the frames the harness
//! hands them (SPEC "Transports"). Each one's manifest is embedded from `manifests/`.

pub mod align;
pub mod compare;
mod raster;
pub mod resize;
pub mod view;

use squint_core::tool::envelope::Provenance;
use squint_core::tool::{Envelope, Manifest, Status, Tool};

/// Every built-in, in no particular order; the registry orders them.
pub fn all() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(view::View::new()),
        Box::new(resize::Resize::new()),
        Box::new(align::Align::new()),
        Box::new(compare::EdgeSimilarity::new()),
        Box::new(compare::ChangeMap::new()),
    ]
}

/// The built-ins' manifests, for listing without a registry.
pub fn manifests() -> Vec<Manifest> {
    all().iter().map(|t| t.manifest().clone()).collect()
}

/// An `invalid_args` envelope: the model's mistake, answered as a tool error.
fn invalid_args(manifest: &Manifest, message: impl Into<String>) -> Envelope {
    Envelope {
        envelope_version: 1,
        status: Status::InvalidArgs,
        summary: message.into(),
        measurements: Vec::new(),
        regions: Vec::new(),
        images: Vec::new(),
        transforms: Vec::new(),
        provenance: Provenance {
            tool: manifest.name.clone(),
            version: manifest.version.clone(),
            duration_ms: 0,
            lock_digest: None,
        },
    }
}

/// An `ok` envelope carrying images.
fn with_images(
    manifest: &Manifest,
    summary: String,
    images: Vec<squint_core::tool::envelope::EnvelopeImage>,
    started: std::time::Instant,
) -> Envelope {
    Envelope {
        envelope_version: 1,
        status: Status::Ok,
        summary,
        measurements: Vec::new(),
        regions: Vec::new(),
        images,
        transforms: Vec::new(),
        provenance: Provenance {
            tool: manifest.name.clone(),
            version: manifest.version.clone(),
            duration_ms: started.elapsed().as_millis() as u64,
            lock_digest: None,
        },
    }
}
