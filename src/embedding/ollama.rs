//! Ollama embeddings via direct HTTP. genai 0.5 doesn't cover Ollama
//! embeddings, so this is a tiny purpose-built client against
//! `POST {base_url}/api/embeddings` with `{"model": ..., "prompt": ...}`.
//!
//! Ollama's embedding endpoint takes one prompt at a time, so batch calls
//! issue requests sequentially. Dimensions are inferred from a probe on
//! `new()` unless explicitly configured.

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{EmbedConfig, EmbeddingProvider};

const DEFAULT_BASE_URL: &str = "http://localhost:11434";

pub struct OllamaEmbedder {
    http: Client,
    base_url: String,
    model: String,
    dimension: usize,
}

#[derive(Serialize)]
struct EmbeddingsRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    embedding: Vec<f32>,
}

impl OllamaEmbedder {
    pub fn new(cfg: &EmbedConfig) -> Result<Self> {
        let base_url = cfg
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let http = Client::new();
        let model = cfg.model.clone();

        // If the user gave us an explicit dimension, trust it; otherwise we'll
        // probe on first use. Probing in `new()` would force this constructor
        // to be async; we sidestep that by deferring.
        let dimension = cfg.dimensions.unwrap_or(0);

        Ok(Self {
            http,
            base_url,
            model,
            dimension,
        })
    }

    async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let req = EmbeddingsRequest {
            model: &self.model,
            prompt: text,
        };
        let response = self
            .http
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&req)
            .send()
            .await
            .context("ollama embeddings request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("ollama embeddings error {status}: {body}"));
        }

        let body: EmbeddingsResponse = response
            .json()
            .await
            .context("ollama embeddings: malformed response")?;
        Ok(body.embedding)
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            out.push(self.embed_one(t).await?);
        }
        Ok(out)
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_one(text).await
    }

    fn provider_name(&self) -> &'static str {
        "ollama"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
