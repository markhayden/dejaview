//! The `Provider` trait: one round trip to a vision model, and the request and response types the
//! harness exchanges with an adapter. Adapters live in `dejaview-providers`; the scripted fake in
//! `dejaview-testkit`. The trait is dyn-compatible and used as `Box<dyn Provider>`.
//!
//! Task 1.1 defines `id`, `send` and the message types. `capabilities`, `wire_schema`,
//! `prepare_images`, `preflight` and `classify` arrive with the adapters (M2) and the state
//! machine (1.7a).

use std::future::Future;
use std::pin::Pin;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Result;

/// One turn to send: the system prompt, the provider history so far, the tools on offer.
#[derive(Debug, Clone)]
pub struct TurnRequest<'a> {
    pub system: &'a str,
    pub history: &'a [Message],
    pub tools: &'a [ToolDefinition],
    pub max_output_tokens: u32,
}

/// A message in the provider history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub role: MessageRole,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

/// The harness's typed view of message content. Adapters map these to their wire shapes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    /// A job or tool image, by id; the adapter attaches the prepared bytes.
    Image {
        image_id: String,
    },
    ToolUse {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
        /// Frame ids of images returned with the result; the adapter attaches the prepared bytes.
        #[serde(default)]
        images: Vec<String>,
    },
}

/// A tool as the provider sees it: name, description and the model schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// What came back from one turn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TurnResponse {
    #[serde(default)]
    pub text: Vec<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub stop_reason: StopReason,
    #[serde(default)]
    pub usage: Usage,
    #[serde(default)]
    pub provider_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    #[default]
    EndTurn,
    ToolUse,
    MaxTokens,
    Refusal,
    Other(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// The future `send` returns: boxed and `Send`, tied to the request's lifetime.
pub type SendFuture<'a> = Pin<Box<dyn Future<Output = Result<TurnResponse>> + Send + 'a>>;

/// One round trip to a model. Implementations never retry or alter history; the state machine
/// owns both.
pub trait Provider: Send + Sync {
    /// The adapter id, for example `anthropic-messages` or `scripted`.
    fn id(&self) -> &str;
    /// Sends one turn and returns the model's response.
    fn send<'a>(&'a self, req: TurnRequest<'a>) -> SendFuture<'a>;
    /// The model's prices and token estimates for the ledger; `None` means no request can be
    /// bounded, and the run is refused before any request.
    fn pricing(&self) -> Option<&dyn crate::ledger::Pricing> {
        None
    }
    /// The image preparation this provider applies (SPEC "Image preparation"); part of the cache
    /// key. `canonical` means the canonical pixels are sent as they are.
    fn prep_version(&self) -> &str {
        "canonical"
    }
    /// Who receives the requests; recorded in the decision header. `None` when nothing leaves
    /// the machine (the scripted provider).
    fn recipient(&self) -> Option<crate::report::Recipient> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_blocks_round_trip_through_json() {
        let block = ContentBlock::ToolUse {
            id: "call_1".into(),
            name: "submit_verdict".into(),
            arguments: serde_json::json!({"schema_version": 1}),
        };
        let text = serde_json::to_string(&block).expect("serialises");
        assert!(text.contains("\"type\":\"tool_use\""));
        let back: ContentBlock = serde_json::from_str(&text).expect("deserialises");
        assert_eq!(back, block);
    }

    #[test]
    fn provider_is_dyn_compatible() {
        fn takes(_: &dyn Provider) {}
        struct Never;
        impl Provider for Never {
            fn id(&self) -> &str {
                "never"
            }
            fn send<'a>(&'a self, _: TurnRequest<'a>) -> SendFuture<'a> {
                Box::pin(async { Err(crate::Error::Provider("never".into())) })
            }
        }
        takes(&Never);
    }
}
