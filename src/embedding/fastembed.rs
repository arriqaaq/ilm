//! Local fastembed-backed embedder. The default — runs on CPU, no network,
//! no API keys. Two model variants supported: BGE-M3 (1024-d, symmetric) and
//! Multilingual E5 Small (384-d, requires `query:`/`passage:` prefixes).

use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use super::{EmbedConfig, EmbeddingProvider};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FastembedModel {
    BgeM3,
    MultilingualE5Small,
}

impl FastembedModel {
    fn parse(name: &str) -> Result<Self> {
        match name {
            "bge-m3" | "BGE-M3" | "bgem3" => Ok(Self::BgeM3),
            "e5-small" | "multilingual-e5-small" | "E5-Small" => Ok(Self::MultilingualE5Small),
            other => {
                anyhow::bail!("unknown fastembed model '{other}'; supported: bge-m3, e5-small")
            }
        }
    }

    fn fastembed_model(self) -> EmbeddingModel {
        match self {
            Self::BgeM3 => EmbeddingModel::BGEM3,
            Self::MultilingualE5Small => EmbeddingModel::MultilingualE5Small,
        }
    }

    fn dimension(self) -> usize {
        match self {
            Self::BgeM3 => 1024,
            Self::MultilingualE5Small => 384,
        }
    }

    fn query_prefix(self) -> &'static str {
        match self {
            Self::BgeM3 => "",
            Self::MultilingualE5Small => "query: ",
        }
    }

    fn passage_prefix(self) -> &'static str {
        match self {
            Self::BgeM3 => "",
            Self::MultilingualE5Small => "passage: ",
        }
    }
}

pub struct FastembedEmbedder {
    inner: Mutex<TextEmbedding>,
    model: FastembedModel,
    model_name: String,
}

impl FastembedEmbedder {
    pub fn new(cfg: &EmbedConfig) -> Result<Self> {
        let model = FastembedModel::parse(&cfg.model)?;
        let inner = TextEmbedding::try_new(
            InitOptions::new(model.fastembed_model()).with_show_download_progress(true),
        )?;
        Ok(Self {
            inner: Mutex::new(inner),
            model,
            model_name: cfg.model.clone(),
        })
    }
}

#[async_trait]
impl EmbeddingProvider for FastembedEmbedder {
    fn dimension(&self) -> usize {
        self.model.dimension()
    }

    async fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let prefix = self.model.passage_prefix();
        let mut model = self.inner.lock().unwrap();
        if prefix.is_empty() {
            Ok(model.embed(texts.to_vec(), None)?)
        } else {
            let prefixed: Vec<String> = texts.iter().map(|t| format!("{prefix}{t}")).collect();
            let refs: Vec<&str> = prefixed.iter().map(|s| s.as_str()).collect();
            Ok(model.embed(refs, None)?)
        }
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        let prefix = self.model.query_prefix();
        let mut model = self.inner.lock().unwrap();
        let mut out = if prefix.is_empty() {
            model.embed(vec![text], None)?
        } else {
            let prefixed = format!("{prefix}{text}");
            model.embed(vec![prefixed.as_str()], None)?
        };
        Ok(out.remove(0))
    }

    fn provider_name(&self) -> &'static str {
        "fastembed"
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}
