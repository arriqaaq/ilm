//! Shared SSE encoding for streaming LLM responses.
//!
//! Five handlers (`/api/ask`, `/api/quran/ask`, `/api/unified/ask`,
//! `/api/books/{id}/chat`, `/api/tafsir/ask` book chat path) all consume a
//! provider-agnostic `TokenStream` and emit Server-Sent Events. This module
//! centralizes that encoding so the handlers no longer know anything about
//! provider wire formats.

use bytes::Bytes;
use futures::Stream;
use futures::StreamExt;

use crate::llm::TokenStream;

/// Convert a `TokenStream` into a stream of SSE-encoded `Bytes`. Empty deltas
/// are suppressed; a final `data: {"done":true}\n\n` event is emitted when
/// the underlying stream signals `done = true`. Errors become `data: {"error":...}` events.
pub fn token_stream_to_sse(
    stream: TokenStream,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send {
    stream.map(|item| {
        let mut sse = String::new();
        match item {
            Ok(ev) => {
                if !ev.delta.is_empty() {
                    sse.push_str(&format!(
                        "data: {}\n\n",
                        serde_json::to_string(&serde_json::json!({ "text": ev.delta })).unwrap()
                    ));
                }
                if ev.done {
                    sse.push_str("data: {\"done\":true}\n\n");
                }
            }
            Err(e) => {
                sse.push_str(&format!(
                    "data: {}\n\n",
                    serde_json::to_string(&serde_json::json!({ "error": e.to_string() })).unwrap()
                ));
            }
        }
        Ok(Bytes::from(sse))
    })
}
