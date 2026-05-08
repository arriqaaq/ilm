//! OpenAI embeddings via genai. Wraps `genai::Client::embed_batch` for
//! `text-embedding-3-small` / `text-embedding-3-large`. Both support the
//! `dimensions` parameter (Matryoshka), exposed via `EmbedConfig.dimensions`.

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use genai::Client;
use genai::embed::EmbedOptions;
use genai::resolver::{AuthData, AuthResolver};

use super::{EmbedConfig, EmbeddingProvider};

pub struct OpenAiEmbedder {
    client: Client,
    model: String,
    /// Pre-namespaced model name: `openai::text-embedding-3-small`.
    namespaced_model: String,
    options: EmbedOptions,
    dimension: usize,
}

impl OpenAiEmbedder {
    pub fn new(cfg: &EmbedConfig) -> Result<Self> {
        let mut builder = Client::builder();

        if let Some(key) = cfg.api_key.clone() {
            let resolver = AuthResolver::from_resolver_fn(
                move |_model_iden: genai::ModelIden| -> genai::resolver::Result<Option<AuthData>> {
                    Ok(Some(AuthData::from_single(key.clone())))
                },
            );
            builder = builder.with_auth_resolver(resolver);
        }

        let client = builder.build();

        let mut options = EmbedOptions::new();
        if let Some(d) = cfg.dimensions {
            options = options.with_dimensions(d);
        }

        // Without an explicit dimension we fall back to OpenAI defaults:
        //   text-embedding-3-small → 1536, text-embedding-3-large → 3072
        // The DB dimension check (`check_embedding_dimension`) will fail loudly
        // if these don't match what's already stored, which is what we want.
        let dimension = cfg.dimensions.unwrap_or_else(|| match cfg.model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 0,
        });

        Ok(Self {
            client,
            model: cfg.model.clone(),
            namespaced_model: format!("openai::{}", cfg.model),
            options,
            dimension,
        })
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        let resp = self
            .client
            .embed_batch(&self.namespaced_model, owned, Some(&self.options))
            .await
            .map_err(|e| anyhow!("openai embed_batch failed: {e}"))?;
        Ok(resp.into_vectors())
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        let resp = self
            .client
            .embed(&self.namespaced_model, text, Some(&self.options))
            .await
            .map_err(|e| anyhow!("openai embed failed: {e}"))?;
        resp.first_vector()
            .cloned()
            .ok_or_else(|| anyhow!("openai embed returned empty response"))
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
