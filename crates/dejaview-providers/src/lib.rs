//! Provider adapters (Anthropic Messages, Gemini generateContent, OpenAI Responses), wire schemas, endpoint profiles, model catalog and the fixtures player.

pub mod catalog;

/// Errors this crate produces. Every public fallible function returns this type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Placeholder until the first task in this crate adds real variants.
    #[error("providers: {0}")]
    Unimplemented(String),
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_displays_its_message() {
        let e = Error::Unimplemented("not yet".into());
        assert_eq!(e.to_string(), "providers: not yet");
    }
}
