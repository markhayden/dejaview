//! Provider adapters (Anthropic Messages, Gemini generateContent, OpenAI Responses), wire schemas,
//! endpoint profiles, model catalog and the fixtures player. Errors are `squint_core::Error`.

pub mod anthropic;
pub mod catalog;
pub mod gemini;
pub mod http;
pub mod openai;
pub mod prepare;
pub mod privacy;
pub mod replay;
pub mod wire;
