//! Core library of the dejaview harness: jobs, images, frames, obligations, checklist, prompt,
//! run state machine, submission, aggregation, decision, journal, report, ledger, cache, and the
//! `Provider` and `Tool` traits. No dependency on other workspace crates.

pub mod aggregate;
pub mod artifacts;
pub mod cache;
pub mod checklist;
pub mod config;
pub mod decision;
pub mod events;
pub mod frames;
pub mod image;
pub mod job;
pub mod journal;
pub mod ledger;
pub mod obligations;
pub mod provider;
pub mod render;
pub mod report;
pub mod retry;
pub mod run;
pub mod submission;
pub mod tool;

/// Errors this crate produces. Every public fallible function returns this type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The config file is missing, unreadable or invalid.
    #[error("config: {0}")]
    Config(String),
    /// The job file is missing, unreadable or invalid.
    #[error("job: {0}")]
    Job(String),
    /// An image could not be read or decoded.
    #[error("image: {0}")]
    Image(String),
    /// The provider answered in a way the harness cannot continue from. Never retried.
    #[error("provider: {0}")]
    Provider(String),
    /// A transport failure from an adapter: status and `retry-after` let the state machine decide
    /// whether to retry (see `retry`).
    #[error("provider: {0}")]
    Transport(retry::TransportError),
    /// The model's submission did not validate.
    #[error("submission: {0}")]
    Submission(String),
    /// The run ended without a submission.
    #[error("no_submission: {0}")]
    NoSubmission(String),
    /// The model's output could not be used: a refusal, text without a call, a truncation, or a
    /// mixed batch, after any repair.
    #[error("model_output: {0}")]
    ModelOutput(String),
    /// A tool crashed, timed out, returned a malformed envelope or breached its bounds.
    #[error("tool: {0}")]
    Tool(String),
    /// A budget (evidence items, spend, turns) would be exceeded.
    #[error("budget: {0}")]
    Budget(String),
    /// The run was cancelled (SIGINT, SIGTERM); the journal ends in `cancelled`.
    #[error("cancelled: {0}")]
    Cancelled(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
}

impl Error {
    /// The spec's error class, as recorded in the journal and report (`error: <class>`).
    pub fn class(&self) -> &'static str {
        match self {
            Error::Config(_) | Error::Job(_) | Error::Image(_) | Error::Toml(_) => "resolve",
            Error::Provider(_) | Error::Transport(_) => "provider",
            Error::Tool(_) => "tool",
            Error::Budget(_) => "budget",
            Error::Cancelled(_) => "cancelled",
            Error::Submission(_) => "submission_invalid",
            Error::NoSubmission(_) => "no_submission",
            Error::ModelOutput(_) => "model_output",
            Error::Io(_) | Error::Json(_) => "io",
        }
    }
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_displays_its_message_and_class() {
        let e = Error::Provider("timed out".into());
        assert_eq!(e.to_string(), "provider: timed out");
        assert_eq!(e.class(), "provider");
    }
}
