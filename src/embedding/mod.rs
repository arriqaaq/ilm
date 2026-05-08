//! Embedding provider abstraction.
//!
//! Application code (`search.rs`, `unified_rag.rs`, bulk indexing) depends on
//! the `EmbeddingProvider` trait via `Arc<dyn EmbeddingProvider>`. Adapters live
//! in submodules: `fastembed` (local), `ollama` (remote HTTP), `openai` (remote
//! via genai). Custom adapters plug in by implementing the trait.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

#[cfg(feature = "advanced")]
pub mod fastembed;
pub mod ollama;
pub mod openai;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Vector dimension this provider/model produces. Stable for the lifetime
    /// of the instance — used by `check_embedding_dimension` to fail fast when
    /// stored embeddings don't match.
    fn dimension(&self) -> usize;

    /// Embed multiple passages (corpus side). Adapters apply provider-specific
    /// quirks internally (E5 `passage:` prefix, Cohere `embedding_type=search_document`, etc.).
    async fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Embed a single query (search side). Mirror of `embed_passages` but for
    /// the asymmetric query/passage embedding case.
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>>;

    fn provider_name(&self) -> &'static str;
    fn model_name(&self) -> &str;
}

/// Which embedding backend to instantiate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum EmbedProviderKind {
    /// Local cross-platform fastembed (no network). The default.
    Fastembed,
    /// OpenAI embeddings (text-embedding-3-*). Uses genai under the hood.
    Openai,
    /// Ollama `/api/embeddings`. Direct HTTP — genai 0.5 doesn't cover Ollama embeddings.
    Ollama,
}

impl EmbedProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fastembed => "fastembed",
            Self::Openai => "openai",
            Self::Ollama => "ollama",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmbedConfig {
    pub provider: EmbedProviderKind,
    /// Provider-specific model name. For fastembed: `bge-m3` or `e5-small`.
    /// For openai: `text-embedding-3-small`/`-large`. For ollama: `nomic-embed-text`, `bge-m3`, etc.
    pub model: String,
    /// Ollama only — defaults to `http://localhost:11434`.
    pub base_url: Option<String>,
    /// OpenAI only.
    pub api_key: Option<String>,
    /// Optional explicit dimension. Required for ollama (no auto-probe), optional for openai
    /// (text-embedding-3-* support reduced dimensions). Ignored for fastembed.
    pub dimensions: Option<usize>,
}

pub fn build_embedder(cfg: &EmbedConfig) -> Result<Arc<dyn EmbeddingProvider>> {
    match cfg.provider {
        #[cfg(feature = "advanced")]
        EmbedProviderKind::Fastembed => Ok(Arc::new(fastembed::FastembedEmbedder::new(cfg)?)),
        #[cfg(not(feature = "advanced"))]
        EmbedProviderKind::Fastembed => {
            anyhow::bail!(
                "fastembed embedder requires the `advanced` cargo feature; rebuild with default features or pick another --embed-provider"
            )
        }
        EmbedProviderKind::Openai => Ok(Arc::new(openai::OpenAiEmbedder::new(cfg)?)),
        EmbedProviderKind::Ollama => Ok(Arc::new(ollama::OllamaEmbedder::new(cfg)?)),
    }
}

/// CLI-friendly enum for the fastembed-only data-prep commands (Ingest,
/// Analyze, IngestQuran). The server's `Serve` command uses the full
/// `EmbedConfig` instead, which can pick any backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum FastembedModelKind {
    /// BAAI/bge-m3 (1024-dim, no prefixes)
    #[value(name = "bge-m3")]
    BgeM3,
    /// intfloat/multilingual-e5-small (384-dim, requires query/passage prefixes)
    #[value(name = "e5-small")]
    #[default]
    MultilingualE5Small,
}

impl FastembedModelKind {
    pub fn dimension(self) -> usize {
        match self {
            Self::BgeM3 => 1024,
            Self::MultilingualE5Small => 384,
        }
    }

    pub fn model_name(self) -> &'static str {
        match self {
            Self::BgeM3 => "bge-m3",
            Self::MultilingualE5Small => "e5-small",
        }
    }

    /// Build a fastembed embedder directly. Used by data-prep CLI commands.
    #[cfg(feature = "advanced")]
    pub fn build(self) -> Result<Arc<dyn EmbeddingProvider>> {
        let cfg = EmbedConfig {
            provider: EmbedProviderKind::Fastembed,
            model: self.model_name().to_string(),
            base_url: None,
            api_key: None,
            dimensions: None,
        };
        build_embedder(&cfg)
    }
}
