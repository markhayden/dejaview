//! A scripted fake provider: answers each turn with the next canned response from a JSON file,
//! so the run loop can be driven through every state offline.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;

use dejaview_core::provider::{Provider, SendFuture, TurnRequest, TurnResponse};

use crate::{Error, Result};

/// Replays a list of `TurnResponse`s in order. Exhausting the script is a provider error, which
/// is what a test wants when the harness asks for more turns than expected.
pub struct ScriptedProvider {
    turns: Mutex<VecDeque<TurnResponse>>,
    /// Every request's history length, for tests that assert the loop's shape.
    pub seen_history_lengths: Mutex<Vec<usize>>,
}

impl ScriptedProvider {
    pub fn new(turns: Vec<TurnResponse>) -> ScriptedProvider {
        ScriptedProvider {
            turns: Mutex::new(turns.into()),
            seen_history_lengths: Mutex::new(Vec::new()),
        }
    }

    /// Loads a JSON array of turn responses.
    pub fn from_path(path: &Path) -> Result<ScriptedProvider> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Script(format!("cannot read {}: {e}", path.display())))?;
        let turns: Vec<TurnResponse> = serde_json::from_str(&text)
            .map_err(|e| Error::Script(format!("{}: {e}", path.display())))?;
        Ok(ScriptedProvider::new(turns))
    }
}

impl Provider for ScriptedProvider {
    fn id(&self) -> &str {
        "scripted"
    }

    fn send<'a>(&'a self, req: TurnRequest<'a>) -> SendFuture<'a> {
        Box::pin(async move {
            if let Ok(mut seen) = self.seen_history_lengths.lock() {
                seen.push(req.history.len());
            }
            let next = self.turns.lock().ok().and_then(|mut t| t.pop_front());
            next.ok_or_else(|| {
                dejaview_core::Error::Provider("scripted provider: script exhausted".into())
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dejaview_core::provider::StopReason;

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        // The scripted provider never yields, so polling once with a no-op waker suffices.
        use std::task::{Context, Poll, Waker};
        let mut cx = Context::from_waker(Waker::noop());
        let mut f = Box::pin(f);
        loop {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    #[test]
    fn replays_turns_in_order_then_errors() {
        let p = ScriptedProvider::new(vec![
            TurnResponse {
                stop_reason: StopReason::ToolUse,
                ..TurnResponse::default_for_test()
            },
            TurnResponse {
                stop_reason: StopReason::EndTurn,
                ..TurnResponse::default_for_test()
            },
        ]);
        let req = TurnRequest {
            system: "",
            history: &[],
            tools: &[],
            max_output_tokens: 1,
        };
        assert_eq!(
            block_on(p.send(req.clone())).expect("turn 1").stop_reason,
            StopReason::ToolUse
        );
        assert_eq!(
            block_on(p.send(req.clone())).expect("turn 2").stop_reason,
            StopReason::EndTurn
        );
        assert!(
            block_on(p.send(req)).is_err(),
            "a third turn exhausts the script"
        );
    }

    trait DefaultForTest {
        fn default_for_test() -> Self;
    }
    impl DefaultForTest for TurnResponse {
        fn default_for_test() -> Self {
            TurnResponse {
                text: vec![],
                tool_calls: vec![],
                stop_reason: StopReason::EndTurn,
                usage: Default::default(),
                provider_request_id: None,
            }
        }
    }
}
