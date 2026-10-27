//! Provider adapters (Anthropic Messages, Gemini generateContent, OpenAI Responses), wire schemas,
//! endpoint profiles, model catalog and the fixtures player. Errors are `dejaview_core::Error`.

pub mod catalog;
pub mod prepare;
pub mod wire;
