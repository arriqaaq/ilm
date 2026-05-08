//! LLM provider abstraction.
//!
//! Application code depends on the `LlmProvider` trait, never on a concrete
//! provider. Adapters live in submodules (`chat` for the genai-backed default,
//! anything else can be plugged in by implementing the trait).
//!
//! Streaming responses are normalized to `TokenEvent`s so HTTP handlers can
//! emit SSE without knowing which provider is in use.

use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::Stream;

pub mod chat;

/// Per-call options. The provider's default model is used when `model` is None.
#[derive(Default, Clone, Debug)]
pub struct ChatOptions {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl ChatOptions {
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            model: Some(model.into()),
            ..Self::default()
        }
    }
}

/// One incremental piece of a streaming chat response.
/// `delta` may be empty on the terminal event; callers should always check `done`.
#[derive(Debug, Clone)]
pub struct TokenEvent {
    pub delta: String,
    pub done: bool,
}

/// Provider-agnostic streaming chat output.
pub type TokenStream = Pin<Box<dyn Stream<Item = Result<TokenEvent>> + Send>>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Non-streaming JSON-mode completion. The provider must coerce the model
    /// into emitting a single JSON object (response_format / format=json /
    /// system-prompt instruction, depending on the backend).
    async fn chat_json(
        &self,
        system: &str,
        user: &str,
        opts: &ChatOptions,
    ) -> Result<serde_json::Value>;

    /// Streaming completion that yields normalized `TokenEvent`s.
    async fn chat_stream(
        &self,
        system: &str,
        user: &str,
        opts: &ChatOptions,
    ) -> Result<TokenStream>;

    fn default_model(&self) -> &str;
    fn provider_name(&self) -> &'static str;
}

/// Which backend to instantiate. Adding a new variant here is the only place
/// the application has to learn about a new provider — call sites are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LlmProviderKind {
    Ollama,
    Openai,
    Anthropic,
}

impl LlmProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: LlmProviderKind,
    /// Required. Provider-specific name (e.g. `llama3.2`, `gpt-4o-mini`, `claude-opus-4-7`).
    pub model: String,
    /// Used by self-hosted backends (Ollama). Defaults to `http://localhost:11434` for Ollama.
    pub base_url: Option<String>,
    /// Required for openai/anthropic.
    pub api_key: Option<String>,
}

/// Build a provider instance from config. The default factory uses the
/// genai-backed adapter; custom adapters can bypass this and instantiate
/// `Arc::new(MyAdapter::new(..)) as Arc<dyn LlmProvider>` directly.
pub fn build_provider(cfg: &LlmConfig) -> Result<Arc<dyn LlmProvider>> {
    Ok(Arc::new(chat::GenAiChatProvider::new(cfg.clone())?))
}
