//! `LlmProvider` adapter backed by the `genai` crate.
//!
//! Maps our `LlmConfig { provider, model, base_url, api_key }` onto a
//! `genai::Client` configured with the right adapter, auth, and (for Ollama)
//! endpoint override. Streaming events are normalized to our `TokenEvent`s.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use futures::StreamExt;
use genai::Client;
use genai::adapter::AdapterKind;
use genai::chat::{
    ChatMessage, ChatOptions as GenAiChatOptions, ChatRequest, ChatResponseFormat, ChatStreamEvent,
};
use genai::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};

use super::{ChatOptions, LlmConfig, LlmProvider, LlmProviderKind, TokenEvent, TokenStream};

/// genai-backed adapter. One instance per server, shared across requests.
pub struct GenAiChatProvider {
    client: Client,
    /// Pre-namespaced model string (e.g. `ollama::llama3.2`, `openai::gpt-4o-mini`,
    /// `anthropic::claude-opus-4-7`). Forces correct adapter routing regardless
    /// of model-name heuristics.
    default_model: String,
    kind: LlmProviderKind,
    /// Just the bare model name (without namespace) for `default_model()` reporting.
    bare_model: String,
}

impl GenAiChatProvider {
    pub fn new(cfg: LlmConfig) -> Result<Self> {
        let kind = cfg.provider;
        let bare_model = cfg.model.clone();
        let default_model = namespaced_model(kind, &cfg.model);

        let mut builder = Client::builder();

        if let Some(key) = cfg.api_key.clone() {
            let resolver = AuthResolver::from_resolver_fn(
                move |_model_iden: genai::ModelIden| -> genai::resolver::Result<Option<AuthData>> {
                    Ok(Some(AuthData::from_single(key.clone())))
                },
            );
            builder = builder.with_auth_resolver(resolver);
        }

        // Ollama endpoint override: genai defaults to http://localhost:11434 for
        // Ollama, but users may run it on a different host/port.
        if matches!(kind, LlmProviderKind::Ollama)
            && let Some(base_url) = cfg.base_url.clone()
        {
            let resolver = ServiceTargetResolver::from_resolver_fn(
                move |mut target: genai::ServiceTarget| -> genai::resolver::Result<genai::ServiceTarget> {
                    if matches!(target.model.adapter_kind, AdapterKind::Ollama) {
                        target.endpoint = Endpoint::from_owned(base_url.clone());
                    }
                    Ok(target)
                },
            );
            builder = builder.with_service_target_resolver(resolver);
        }

        let client = builder.build();

        Ok(Self {
            client,
            default_model,
            kind,
            bare_model,
        })
    }

    /// Resolve the effective namespaced model string for a call. If the caller
    /// supplied an override in `ChatOptions`, namespace it for the same
    /// provider as the default; otherwise use the default.
    fn resolve_model(&self, opts: &ChatOptions) -> String {
        match &opts.model {
            Some(m) => namespaced_model(self.kind, m),
            None => self.default_model.clone(),
        }
    }

    fn build_genai_opts(&self, opts: &ChatOptions, json_mode: bool) -> GenAiChatOptions {
        let mut g = GenAiChatOptions::default();
        if let Some(t) = opts.temperature {
            g = g.with_temperature(t as f64);
        }
        if let Some(m) = opts.max_tokens {
            g = g.with_max_tokens(m);
        }
        if json_mode {
            g = g.with_response_format(ChatResponseFormat::JsonMode);
        }
        g
    }
}

#[async_trait]
impl LlmProvider for GenAiChatProvider {
    async fn chat_json(
        &self,
        system: &str,
        user: &str,
        opts: &ChatOptions,
    ) -> Result<serde_json::Value> {
        let req = ChatRequest::new(vec![
            ChatMessage::system(system.to_string()),
            ChatMessage::user(user.to_string()),
        ]);
        let g_opts = self.build_genai_opts(opts, true);
        let model = self.resolve_model(opts);

        let response = self
            .client
            .exec_chat(&model, req, Some(&g_opts))
            .await
            .map_err(|e| anyhow!("LLM chat_json failed: {e}"))?;

        let text = response.first_text().unwrap_or("{}");
        serde_json::from_str(text).with_context(|| format!("LLM returned non-JSON content: {text}"))
    }

    async fn chat_stream(
        &self,
        system: &str,
        user: &str,
        opts: &ChatOptions,
    ) -> Result<TokenStream> {
        let req = ChatRequest::new(vec![
            ChatMessage::system(system.to_string()),
            ChatMessage::user(user.to_string()),
        ]);
        let g_opts = self.build_genai_opts(opts, false);
        let model = self.resolve_model(opts);

        let resp = self
            .client
            .exec_chat_stream(&model, req, Some(&g_opts))
            .await
            .map_err(|e| anyhow!("LLM chat_stream failed: {e}"))?;

        let mapped = resp.stream.map(|item| match item {
            Ok(ChatStreamEvent::Chunk(chunk)) => Ok(TokenEvent {
                delta: chunk.content,
                done: false,
            }),
            Ok(ChatStreamEvent::Start)
            | Ok(ChatStreamEvent::ReasoningChunk(_))
            | Ok(ChatStreamEvent::ThoughtSignatureChunk(_))
            | Ok(ChatStreamEvent::ToolCallChunk(_)) => Ok(TokenEvent {
                delta: String::new(),
                done: false,
            }),
            Ok(ChatStreamEvent::End(_)) => Ok(TokenEvent {
                delta: String::new(),
                done: true,
            }),
            Err(e) => Err(anyhow!("LLM stream error: {e}")),
        });

        Ok(Box::pin(mapped))
    }

    fn default_model(&self) -> &str {
        &self.bare_model
    }

    fn provider_name(&self) -> &'static str {
        self.kind.as_str()
    }
}

fn namespaced_model(kind: LlmProviderKind, model: &str) -> String {
    // genai's namespace parser uses the lowercase AdapterKind name; passing it
    // explicitly avoids the model-name prefix heuristics misrouting (e.g.
    // `command-r7b-arabic` would otherwise resolve to Cohere).
    format!("{}::{}", kind.as_str(), model)
}

// `Arc<dyn LlmProvider>` requires Send + Sync; genai::Client is both.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GenAiChatProvider>();
};

// Suppress unused-import lint when this file is the only `Arc` user.
const _: Option<Arc<dyn LlmProvider>> = None;
