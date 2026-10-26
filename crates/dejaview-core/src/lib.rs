//! Core library of the dejaview harness: jobs, images, frames, obligations, checklist, prompt,
//! run state machine, submission, aggregation, decision, journal, report, ledger, cache, and the
//! `Provider` and `Tool` traits. No dependency on other workspace crates.

pub mod aggregate;
pub mod checklist;
pub mod config;
pub mod frames;
pub mod image;
pub mod job;
pub mod journal;
pub mod obligations;
pub mod provider;
pub mod report;
pub mod run;
pub mod submission;

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
    /// The provider failed or answered in a way the harness cannot continue from.
    #[error("provider: {0}")]
    Provider(String),
    /// The model's submission did not validate.
    #[error("submission: {0}")]
    Submission(String),
    /// The run ended without a submission.
    #[error("no_submission: {0}")]
    NoSubmission(String),
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
            Error::Provider(_) => "provider",
            Error::Submission(_) => "submission_invalid",
            Error::NoSubmission(_) => "no_submission",
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
