//! Test support shared by every crate: the scripted fake provider, the fake tool and the fake MCP
//! server. A normal dependency of the `dejaview` binary only behind its `testkit` feature.

pub mod provider;
pub mod tool;

pub use provider::ScriptedProvider;
pub use tool::ScriptedTool;

/// Errors this crate produces.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("script: {0}")]
    Script(String),
}

pub type Result<T> = std::result::Result<T, Error>;
