use anyhow::Result;
use fastembed::{
    EmbeddingModel, InitOptions, RerankInitOptions, RerankerModel, TextEmbedding, TextRerank,
};
use std::sync::{Arc, Mutex};
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;
use crate::rag::OllamaClient;

const BATCH_SIZE: usize = 64;

/// Supported embedding models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum EmbedModel {
    /// BAAI/bge-m3 (1024-dim, no prefixes)
    #[value(name = "bge-m3")]
    BgeM3,
    /// intfloat/multilingual-e5-small (384-dim, requires query/passage prefixes)
    #[value(name = "e5-small")]
    #[default]
    MultilingualE5Small,
}

impl EmbedModel {
    pub fn fastembed_model(&self) -> EmbeddingModel {
        match self {
            Self::BgeM3 => EmbeddingModel::BGEM3,
            Self::MultilingualE5Small => EmbeddingModel::MultilingualE5Small,
        }
    }

    pub fn dimension(&self) -> usize {
        match self {
            Self::BgeM3 => 1024,
            Self::MultilingualE5Small => 384,
        }
    }

    fn query_prefix(&self) -> &'static str {
        match self {
            Self::BgeM3 => "",
            Self::MultilingualE5Small => "query: ",
        }
    }

    fn passage_prefix(&self) -> &'static str {
        match self {
            Self::BgeM3 => "",
            Self::MultilingualE5Small => "passage: ",
        }
    }
}

pub struct Embedder {
    model: Mutex<TextEmbedding>,
    config: EmbedModel,
}

impl Embedder {
    pub fn new(config: EmbedModel) -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(config.fastembed_model()).with_show_download_progress(true),
        )?;
        Ok(Self {
            model: Mutex::new(model),
            config,
        })
    }

    pub fn dimension(&self) -> usize {
        self.config.dimension()
    }

    /// Embed passages (applies passage prefix for models that need it).
    pub fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let prefix = self.config.passage_prefix();
        let mut model = self.model.lock().unwrap();
        if prefix.is_empty() {
            let embeddings = model.embed(texts, None)?;
            Ok(embeddings)
        } else {
            let prefixed: Vec<String> = texts.iter().map(|t| format!("{prefix}{t}")).collect();
            let refs: Vec<&str> = prefixed.iter().map(|s| s.as_str()).collect();
            let embeddings = model.embed(refs, None)?;
            Ok(embeddings)
        }
    }

    /// Embed a single query (applies query prefix for models that need it).
    pub fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let prefix = self.config.query_prefix();
        let mut model = self.model.lock().unwrap();
        if prefix.is_empty() {
            let mut embeddings = model.embed(vec![text], None)?;
            Ok(embeddings.remove(0))
        } else {
            let prefixed = format!("{prefix}{text}");
            let mut embeddings = model.embed(vec![prefixed.as_str()], None)?;
            Ok(embeddings.remove(0))
        }
    }
}

/// Which reranker backend to use at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RerankBackendKind {
    /// Cross-encoder via fastembed (local, fast, BAAI/bge-reranker-v2-m3).
    Fastembed,
    /// LLM via Ollama (slower, needs a running Ollama server).
    Ollama,
}

/// Cross-encoder reranker backed by fastembed.
pub struct FastembedReranker {
    model: Mutex<TextRerank>,
}

impl FastembedReranker {
    pub fn new() -> Result<Self> {
        let model = TextRerank::try_new(
            RerankInitOptions::new(RerankerModel::BGERerankerV2M3)
                .with_show_download_progress(true),
        )?;
        Ok(Self {
            model: Mutex::new(model),
        })
    }

    fn rerank(&self, query: &str, passages: &[&str]) -> Result<Vec<f32>> {
        let mut model = self.model.lock().unwrap();
        let results = model.rerank(query, passages, false, Some(BATCH_SIZE))?;
        let mut scores = vec![0.0f32; passages.len()];
        for r in results {
            scores[r.index] = r.score;
        }
        Ok(scores)
    }
}

/// Runtime reranker. Either a local cross-encoder or a remote LLM. Both
/// implement the same `rerank(query, passages) -> Vec<f32>` contract; scores
/// are raw model outputs and only meaningful as a within-query ranking signal.
pub enum RerankerBackend {
    Fastembed(FastembedReranker),
    Ollama {
        client: Arc<OllamaClient>,
        model: String,
    },
}

impl RerankerBackend {
    pub async fn rerank(&self, query: &str, passages: &[&str]) -> Result<Vec<f32>> {
        match self {
            Self::Fastembed(r) => r.rerank(query, passages),
            Self::Ollama { client, model } => ollama_rerank(client, model, query, passages).await,
        }
    }
}

/// Listwise Ollama reranker. Batches passages in groups to keep prompts
/// manageable, asks the model to return JSON scores per passage, stitches
/// them back together in input order. Unscored passages default to 0.
///
/// Scores are 0.0–1.0 relevance judgments. Not calibrated across queries —
/// only use for ranking within a single query's candidate set.
async fn ollama_rerank(
    client: &OllamaClient,
    model: &str,
    query: &str,
    passages: &[&str],
) -> Result<Vec<f32>> {
    const BATCH: usize = 10;
    const SYSTEM: &str = "You are a relevance judge for Islamic hadith and Quranic text search. \
        For each passage, rate how well it answers or relates to the user's query on a 0.0 to 1.0 scale \
        where 1.0 is a direct, on-topic answer and 0.0 is unrelated. \
        Return ONLY valid JSON of the form {\"scores\": [<float>, ...]} \
        with exactly as many scores as input passages, in the same order. No prose.";

    let mut scores = vec![0.0f32; passages.len()];
    for (batch_idx, chunk) in passages.chunks(BATCH).enumerate() {
        let mut user = format!("Query: {query}\n\nPassages:\n");
        for (i, p) in chunk.iter().enumerate() {
            // Keep per-passage text bounded to avoid huge prompts.
            let trimmed: String = p.chars().take(600).collect();
            user.push_str(&format!("[{}] {}\n", i + 1, trimmed));
        }
        user.push_str(&format!(
            "\nReturn JSON: {{\"scores\": [<{} floats between 0 and 1>]}}",
            chunk.len()
        ));

        let parsed = client.chat_json(SYSTEM, &user, Some(model)).await?;
        let arr = parsed
            .get("scores")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for (i, v) in arr.iter().enumerate() {
            if let Some(f) = v.as_f64() {
                let out_idx = batch_idx * BATCH + i;
                if out_idx < scores.len() {
                    scores[out_idx] = (f as f32).clamp(0.0, 1.0);
                }
            }
        }
    }
    Ok(scores)
}

/// Check that existing embeddings (if any) match the expected dimension.
/// Returns an error with instructions if there's a mismatch.
pub async fn check_embedding_dimension(db: &Surreal<Db>, expected_dim: usize) -> Result<()> {
    #[derive(Debug, SurrealValue)]
    struct EmbedProbe {
        embedding: Option<Vec<f32>>,
    }
    let mut res = db
        .query("SELECT embedding FROM hadith WHERE embedding IS NOT NONE LIMIT 1")
        .await?;
    let probes: Vec<EmbedProbe> = res.take(0)?;
    if let Some(probe) = probes.first()
        && let Some(ref emb) = probe.embedding
        && emb.len() != expected_dim
    {
        anyhow::bail!(
            "Existing embeddings have dimension {} but selected model produces dimension {}.\n\
                     To switch models, clean your data directory and re-ingest:\n  \
                     rm -rf db_data\n  \
                     hadith ingest --embed-model <model> --file data/semantic_hadith.json\n  \
                     hadith ingest-quran --embed-model <model> --file data/quran.csv",
            emb.len(),
            expected_dim,
        );
    }
    Ok(())
}

/// Generate embeddings for all hadiths that don't have one yet.
pub async fn embed_all_hadiths(db: &Surreal<Db>, embedder: &Embedder) -> Result<()> {
    // Get hadiths without embeddings
    let mut response = db
        .query("SELECT id, hadith_number, text_ar, text_en, narrator_text FROM hadith WHERE embedding IS NONE")
        .await?;
    let hadiths: Vec<HadithForEmbed> = response.take(0)?;

    let total = hadiths.len();

    let pb = indicatif::ProgressBar::new(total as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.green/black} {pos}/{len} embeddings ({eta})")
            .unwrap(),
    );

    for chunk in hadiths.chunks(BATCH_SIZE) {
        let texts: Vec<String> = chunk
            .iter()
            .map(|h| {
                let narrator = h.narrator_text.as_deref().unwrap_or("");
                let text = match (h.text_ar.as_deref(), h.text_en.as_deref()) {
                    (Some(ar), Some(en)) => format!("{} {}", ar, en),
                    (Some(ar), None) => ar.to_string(),
                    (None, Some(en)) => en.to_string(),
                    (None, None) => String::new(),
                };
                format!("{} {}", narrator, text)
            })
            .collect();

        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let embeddings = embedder.embed(&text_refs)?;

        let futs: Vec<_> = chunk
            .iter()
            .zip(embeddings.into_iter())
            .filter_map(|(hadith, embedding)| {
                hadith.id.as_ref().map(|id| {
                    db.query("UPDATE $id SET embedding = $embedding")
                        .bind(("id", id.clone()))
                        .bind(("embedding", embedding))
                })
            })
            .collect();

        for fut in futs {
            fut.await?;
        }

        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("done");
    println!("   ✓ {} embeddings generated", total);
    Ok(())
}

#[derive(Debug, SurrealValue)]
struct HadithForEmbed {
    id: Option<RecordId>,
    #[allow(dead_code)]
    hadith_number: i64,
    text_ar: Option<String>,
    text_en: Option<String>,
    narrator_text: Option<String>,
}
