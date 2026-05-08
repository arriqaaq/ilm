//! Reranker backends and bulk-indexing helpers.
//!
//! The actual embedding model lives in `crate::embedding` (provider-agnostic
//! trait + adapters). This file is what's left: rerankers (which can be a
//! local cross-encoder or any LLM provider) and the indexing helpers that
//! wire embeddings into SurrealDB.

use anyhow::Result;
#[cfg(feature = "advanced")]
use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
use std::sync::Arc;
#[cfg(feature = "advanced")]
use std::sync::Mutex;
#[cfg(feature = "advanced")]
use surrealdb::Surreal;
#[cfg(feature = "advanced")]
use surrealdb::types::{RecordId, SurrealValue};

#[cfg(feature = "advanced")]
use crate::db::Db;
#[cfg(feature = "advanced")]
use crate::embedding::EmbeddingProvider;
use crate::llm::{ChatOptions, LlmProvider};

#[cfg(feature = "advanced")]
const BATCH_SIZE: usize = 64;

// ── Reranker ─────────────────────────────────────────────────────────────────

/// Which reranker backend to use at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RerankBackendKind {
    /// Cross-encoder via fastembed (local, fast, BAAI/bge-reranker-v2-m3).
    Fastembed,
    /// LLM relevance judge via the configured `--llm-provider`.
    /// Slower; same provider as the main chat client.
    Llm,
}

#[cfg(feature = "advanced")]
pub struct FastembedReranker {
    model: Mutex<TextRerank>,
}

#[cfg(feature = "advanced")]
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
    #[cfg(feature = "advanced")]
    Fastembed(FastembedReranker),
    Llm {
        provider: Arc<dyn LlmProvider>,
        /// Optional override; falls back to the provider's default model.
        model: Option<String>,
    },
}

impl RerankerBackend {
    pub async fn rerank(&self, query: &str, passages: &[&str]) -> Result<Vec<f32>> {
        match self {
            #[cfg(feature = "advanced")]
            Self::Fastembed(r) => r.rerank(query, passages),
            Self::Llm { provider, model } => {
                llm_rerank(provider.as_ref(), model.as_deref(), query, passages).await
            }
        }
    }
}

/// Listwise LLM-backed reranker. Batches passages in groups of 10, asks the
/// model for a JSON `{"scores": [...]}` per batch, and stitches results back
/// in input order. Unscored passages default to 0. Provider-agnostic — works
/// against Ollama, OpenAI, Anthropic, or any custom `LlmProvider`.
///
/// Scores are 0.0–1.0 relevance judgments. Not calibrated across queries —
/// only use for ranking within a single query's candidate set.
async fn llm_rerank(
    provider: &dyn LlmProvider,
    model: Option<&str>,
    query: &str,
    passages: &[&str],
) -> Result<Vec<f32>> {
    const BATCH: usize = 10;
    const SYSTEM: &str = "You are a relevance judge for Islamic hadith and Quranic text search. \
        For each passage, rate how well it answers or relates to the user's query on a 0.0 to 1.0 scale \
        where 1.0 is a direct, on-topic answer and 0.0 is unrelated. \
        Return ONLY valid JSON of the form {\"scores\": [<float>, ...]} \
        with exactly as many scores as input passages, in the same order. No prose.";

    let opts = ChatOptions {
        model: model.map(str::to_string),
        ..Default::default()
    };

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

        let parsed = provider.chat_json(SYSTEM, &user, &opts).await?;
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

// ── Embedding helpers (advanced-only) ────────────────────────────────────────

/// Check that existing embeddings (if any) match the embedder's dimension.
/// Returns an error with instructions if there's a mismatch — switching embed
/// providers/models requires re-ingestion.
#[cfg(feature = "advanced")]
pub async fn check_embedding_dimension(
    db: &Surreal<Db>,
    embedder: &dyn EmbeddingProvider,
) -> Result<()> {
    let expected_dim = embedder.dimension();
    if expected_dim == 0 {
        // Embedder didn't declare a dimension up-front (e.g. Ollama probe pending).
        // Skip the check — first embed call will tell us.
        return Ok(());
    }
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
            "Existing embeddings have dimension {} but selected embedder ({}/{}) produces dimension {}.\n\
                     To switch models, clean your data directory and re-ingest:\n  \
                     rm -rf db_data\n  \
                     hadith ingest --file data/semantic_hadith.json\n  \
                     hadith ingest-quran --file data/quran.csv",
            emb.len(),
            embedder.provider_name(),
            embedder.model_name(),
            expected_dim,
        );
    }
    Ok(())
}

/// Generate embeddings for all hadiths that don't have one yet.
#[cfg(feature = "advanced")]
pub async fn embed_all_hadiths(db: &Surreal<Db>, embedder: &dyn EmbeddingProvider) -> Result<()> {
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
        let embeddings = embedder.embed_passages(&text_refs).await?;

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

#[cfg(feature = "advanced")]
#[derive(Debug, SurrealValue)]
struct HadithForEmbed {
    id: Option<RecordId>,
    #[allow(dead_code)]
    hadith_number: i64,
    text_ar: Option<String>,
    text_en: Option<String>,
    narrator_text: Option<String>,
}
