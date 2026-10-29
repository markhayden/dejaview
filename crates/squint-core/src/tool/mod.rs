//! Tools: the manifest every tool exposes, the result envelope every tool returns, the context
//! the harness injects, and the `Tool` trait built-ins implement (SPEC "Tools"). The MCP
//! transport (task 1.13) speaks the same manifest and envelope.

pub mod envelope;
pub mod manifest;
pub mod projection;
pub mod registry;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use envelope::{Envelope, Status};
pub use manifest::{Bounds, CostTier, Manifest};
pub use projection::Projected;
pub use registry::ToolRegistry;

use crate::Result;

/// The harness-owned object injected into every tool call after the model's arguments validated.
/// The model never sees or supplies it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SquintContext {
    pub run_id: String,
    pub call_id: String,
    /// Canonical images the call named through `image_args`, by image id.
    pub images: BTreeMap<String, ImageHandle>,
    /// Frames the call named through `frame_args`, by frame id, with their transform chains.
    pub frames: BTreeMap<String, FrameHandle>,
    /// Hints from the rules for this tool, as declared in `accepts_hints`.
    #[serde(default)]
    pub hints: serde_json::Value,
    /// The run directory, for transports that spawn a process there; never sent to a tool.
    #[serde(skip)]
    pub run_dir: Option<std::path::PathBuf>,
}

/// How a tool reaches an image's canonical pixels: a lossless PNG by path or inline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageHandle {
    pub media_type: String,
    pub width: u32,
    pub height: u32,
    pub projection: crate::job::Projection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
}

/// A derived frame delivered to a tool: its root, parent, transform, the chain of frames down to
/// the root, and lossless pixels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrameHandle {
    pub root: String,
    pub projection: crate::job::Projection,
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<serde_json::Value>,
    /// The frame and each ancestor with how it sits in its parent, ending at the raw root, so a
    /// pack can reproduce any location without state of its own.
    #[serde(default)]
    pub chain: Vec<crate::frames::ChainLink>,
    pub pixels: ImageHandle,
}

/// The key under which the context travels inside tool arguments. The model may never set it.
pub const CONTEXT_KEY: &str = "_squint";

/// A deterministic tool. Built-ins implement this directly; MCP tools are wrapped by the client.
pub trait Tool: Send + Sync {
    fn manifest(&self) -> &Manifest;
    /// Runs the tool on validated model arguments plus the injected context.
    fn call(&self, args: &serde_json::Value, ctx: &SquintContext) -> Result<Envelope>;
}

/// What happens to a model's tool call before execution.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgCheck {
    /// Arguments are acceptable; execute.
    Ok,
    /// Returned to the model as a tool error; not an operational failure.
    InvalidArgs(String),
}

/// Checks a model's arguments against the manifest: no harness-owned key, and the model schema's
/// required properties present (full JSON Schema validation joins in task 1.6a's validator).
pub fn check_model_args(manifest: &Manifest, args: &serde_json::Value) -> ArgCheck {
    let Some(obj) = args.as_object() else {
        return ArgCheck::InvalidArgs("arguments must be an object".into());
    };
    if obj.contains_key(CONTEXT_KEY) {
        return ArgCheck::InvalidArgs(format!(
            "{CONTEXT_KEY} is supplied by the harness and may not appear in the model's arguments"
        ));
    }
    if let Some(required) = manifest
        .model_schema
        .get("required")
        .and_then(|r| r.as_array())
    {
        for key in required.iter().filter_map(|k| k.as_str()) {
            if !obj.contains_key(key) {
                return ArgCheck::InvalidArgs(format!("missing required argument {key:?}"));
            }
        }
    }
    let closed =
        manifest.model_schema.get("additionalProperties") == Some(&serde_json::Value::Bool(false));
    if let Some(props) = closed
        .then(|| {
            manifest
                .model_schema
                .get("properties")
                .and_then(|p| p.as_object())
        })
        .flatten()
    {
        for key in obj.keys() {
            if !props.contains_key(key) {
                return ArgCheck::InvalidArgs(format!("unknown argument {key:?}"));
            }
        }
    }
    ArgCheck::Ok
}

/// Builds the full call arguments: the model's validated arguments plus the injected context.
pub fn inject_context(args: &serde_json::Value, ctx: &SquintContext) -> Result<serde_json::Value> {
    let mut full = args.clone();
    let obj = full
        .as_object_mut()
        .ok_or_else(|| crate::Error::Tool("arguments must be an object".into()))?;
    obj.insert(CONTEXT_KEY.to_string(), serde_json::to_value(ctx)?);
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest::from_toml(&crate::fixture("tools/regions.toml")).expect("manifest")
    }

    #[test]
    fn model_args_with_the_context_key_are_invalid() {
        let m = manifest();
        let args = serde_json::json!({ "image": "candidate", "against": "aligned:source:t1", "_squint": {} });
        assert!(
            matches!(check_model_args(&m, &args), ArgCheck::InvalidArgs(msg) if msg.contains("_squint"))
        );
    }

    #[test]
    fn missing_and_unknown_arguments_are_invalid() {
        let m = manifest();
        assert!(
            matches!(check_model_args(&m, &serde_json::json!({ "image": "candidate" })), ArgCheck::InvalidArgs(msg) if msg.contains("against"))
        );
        assert!(matches!(
            check_model_args(&m, &serde_json::json!({ "image": "candidate", "against": "x", "extra": 1 })),
            ArgCheck::InvalidArgs(msg) if msg.contains("extra")
        ));
        assert_eq!(
            check_model_args(
                &m,
                &serde_json::json!({ "image": "candidate", "against": "x" })
            ),
            ArgCheck::Ok
        );
    }

    #[test]
    fn inject_context_adds_the_harness_key() {
        let ctx = SquintContext {
            run_id: "r".into(),
            call_id: "call:1".into(),
            ..Default::default()
        };
        let full =
            inject_context(&serde_json::json!({ "image": "candidate" }), &ctx).expect("inject");
        assert_eq!(full["_squint"]["call_id"], "call:1");
        assert_eq!(full["image"], "candidate");
    }
}
