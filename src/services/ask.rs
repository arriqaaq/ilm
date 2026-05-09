//! Ask / RAG services.
//!
//! Combines the agentic two-phase classify-then-execute pipeline (formerly
//! `crate::agentic_rag`) with the per-scope semantic-RAG fallbacks (formerly
//! `crate::unified_rag`). Returns sources + a token stream; the HTTP layer
//! wraps that in SSE, the MCP layer drains it into a single string.
//!
//! ## Why two phases?
//!
//! For structured questions ("how many hadiths did Abu Huraira narrate?") we
//! want exact database results, not LLM guesses. The LLM classifies the
//! question into a `QueryIntent`, the corresponding `services::narrator` tool
//! returns a verified `ToolOutput`, and the LLM streams an answer grounded
//! in that exact data. For unstructured questions ("what does Islam say
//! about patience?") we fall back to scope-appropriate semantic RAG.

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;
use crate::embedding::EmbeddingProvider;
use crate::llm::{ChatOptions, LlmProvider, TokenStream};
use crate::models::{HadithSearchResult, record_id_string};
use crate::quran::models::AyahSearchResult;
use crate::quran::surah_name;
use crate::services::classify::{QueryIntent, classify};
use crate::services::narrator::{self, ApiNarratorSource};

// ── Public surface: scope + agentic result ──

/// Which corpus the caller wants the Ask pipeline scoped to.
/// Structured narrator intents fire under `Hadith` and `Both`; under `Quran`
/// we skip classification entirely and go straight to the scoped semantic
/// fallback (none of the current structured intents apply to ayahs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskScope {
    Hadith,
    Quran,
    Both,
}

/// Result of the agentic RAG pipeline.
pub enum AgenticResult {
    /// Structured DB query path — we have exact data, stream the answer.
    Structured {
        narrator_sources: Vec<ApiNarratorSource>,
        hadith_sources: Vec<HadithSearchResult>,
        token_stream: TokenStream,
    },
    /// Fallback to scope-appropriate semantic RAG. Empty source vecs for
    /// corpora outside the requested scope (e.g. `ayah_sources = vec![]`
    /// under `AskScope::Hadith`).
    Semantic {
        ayah_sources: Vec<AyahSearchResult>,
        hadith_sources: Vec<HadithSearchResult>,
        token_stream: TokenStream,
    },
}

// ── Tunables ──

pub(crate) const CONTEXT_AYAH_COUNT: usize = 4;
pub(crate) const CONTEXT_HADITH_COUNT: usize = 4;
const CONTEXT_HADITH_COUNT_SOLO: usize = 6;
const CONTEXT_AYAH_COUNT_SOLO: usize = 6;
const MAX_TAFSIR_CHARS: usize = 1000;
const MAX_TAFSIR_CHARS_SOLO: usize = 2000;

const STRUCTURED_SYSTEM_PREFIX: &str = "\
You are a knowledgeable Islamic hadith scholar. Answer using ONLY the verified database results below.\n\
These numbers are exact counts from the database — do not estimate, round, or guess.\n\
Cite specific data points (names, numbers, generations) from the results.\n\
If the data doesn't answer the question, say so honestly.\n\n";

// ── Agentic entry point ──

/// Agentic RAG: classify intent, run structured queries or fall back to semantic RAG.
pub async fn ask_agentic(
    provider: &dyn LlmProvider,
    db: &Surreal<Db>,
    embedder: &dyn EmbeddingProvider,
    question: &str,
    opts: &ChatOptions,
    scope: AskScope,
) -> Result<AgenticResult> {
    // Quran scope: none of the current structured intents apply to ayahs,
    // so skip classification (saves ~500ms) and go straight to the
    // Quran-only semantic fallback.
    if scope == AskScope::Quran {
        return fallback_semantic(provider, db, embedder, question, opts, scope).await;
    }

    // Phase 1: Classify the user's question into a structured intent.
    let intent = match classify(provider, question, opts).await {
        Ok(intent) => intent,
        Err(e) => {
            tracing::warn!("Classification failed, falling back to semantic: {e}");
            QueryIntent::ContentQuery
        }
    };

    // ContentQuery → fall back to existing semantic vector search RAG.
    if matches!(intent, QueryIntent::ContentQuery) {
        return fallback_semantic(provider, db, embedder, question, opts, scope).await;
    }

    // Phase 2: Execute structured DB queries based on the classified intent.
    let tool_result = match &intent {
        QueryIntent::NarratorInfo { name } => {
            let Some(n) = narrator::resolve_narrator(db, name).await? else {
                tracing::info!("Narrator '{name}' not found, falling back to semantic");
                return fallback_semantic(provider, db, embedder, question, opts, scope).await;
            };
            narrator::narrator_info(db, &n).await?
        }
        QueryIntent::NarratorCount { name, book } => {
            let Some(n) = narrator::resolve_narrator(db, name).await? else {
                return fallback_semantic(provider, db, embedder, question, opts, scope).await;
            };
            narrator::count_hadiths(db, &n, book.as_deref()).await?
        }
        QueryIntent::NarratorTeachers { name } => {
            let Some(n) = narrator::resolve_narrator(db, name).await? else {
                return fallback_semantic(provider, db, embedder, question, opts, scope).await;
            };
            narrator::narrator_teachers(db, &n).await?
        }
        QueryIntent::NarratorStudents { name } => {
            let Some(n) = narrator::resolve_narrator(db, name).await? else {
                return fallback_semantic(provider, db, embedder, question, opts, scope).await;
            };
            narrator::narrator_students(db, &n).await?
        }
        QueryIntent::NarratorHadiths { name } => {
            let Some(n) = narrator::resolve_narrator(db, name).await? else {
                return fallback_semantic(provider, db, embedder, question, opts, scope).await;
            };
            narrator::narrator_hadiths(db, &n, 10).await?
        }
        QueryIntent::IsnadSearch { narrators, ordered } => {
            let mut resolved = Vec::new();
            for name in narrators {
                match narrator::resolve_narrator(db, name).await? {
                    Some(n) => resolved.push(n),
                    None => {
                        tracing::info!("Narrator '{name}' not found in isnad search, falling back");
                        return fallback_semantic(provider, db, embedder, question, opts, scope)
                            .await;
                    }
                }
            }
            narrator::isnad_search_tool(db, &resolved, *ordered, 10).await?
        }
        QueryIntent::CommonNarrators { name1, name2 } => {
            let Some(n1) = narrator::resolve_narrator(db, name1).await? else {
                return fallback_semantic(provider, db, embedder, question, opts, scope).await;
            };
            let Some(n2) = narrator::resolve_narrator(db, name2).await? else {
                return fallback_semantic(provider, db, embedder, question, opts, scope).await;
            };
            narrator::common_narrators_tool(db, &n1, &n2).await?
        }
        QueryIntent::ContentQuery => unreachable!(),
    };

    // Phase 3: Stream the LLM answer, grounded in exact database results.
    let system_prompt = format!("{STRUCTURED_SYSTEM_PREFIX}{}", tool_result.context);
    let token_stream = provider.chat_stream(&system_prompt, question, opts).await?;

    Ok(AgenticResult::Structured {
        narrator_sources: tool_result.narrator_sources,
        hadith_sources: tool_result.hadith_sources,
        token_stream,
    })
}

/// Fallback to scope-appropriate semantic RAG.
async fn fallback_semantic(
    provider: &dyn LlmProvider,
    db: &Surreal<Db>,
    embedder: &dyn EmbeddingProvider,
    question: &str,
    opts: &ChatOptions,
    scope: AskScope,
) -> Result<AgenticResult> {
    match scope {
        AskScope::Hadith => {
            let (hadith_sources, token_stream) =
                ask_hadith_only(provider, db, embedder, question, opts).await?;
            Ok(AgenticResult::Semantic {
                ayah_sources: vec![],
                hadith_sources,
                token_stream,
            })
        }
        AskScope::Quran => {
            let (ayah_sources, token_stream) =
                ask_quran_only(provider, db, embedder, question, opts).await?;
            Ok(AgenticResult::Semantic {
                ayah_sources,
                hadith_sources: vec![],
                token_stream,
            })
        }
        AskScope::Both => {
            let (ayah_sources, hadith_sources, token_stream) =
                ask_unified(provider, db, embedder, question, opts).await?;
            Ok(AgenticResult::Semantic {
                ayah_sources,
                hadith_sources,
                token_stream,
            })
        }
    }
}

// ── Retrieval + context builders (shared between the three scoped ask methods) ──

/// Retrieve semantically-similar hadiths and format them as an LLM context block
/// headed by `## Relevant Hadiths:`. Batch-fetches isnad chains to avoid N+1.
/// Swallows retrieval errors (returns empty sources + header-only context).
async fn retrieve_and_build_hadith_context(
    db: &Surreal<Db>,
    embedder: &dyn EmbeddingProvider,
    question: &str,
    k: usize,
) -> (Vec<HadithSearchResult>, String) {
    let hadith_sources = crate::search::search_hadiths_semantic(db, embedder, question, k)
        .await
        .unwrap_or_default();

    #[derive(Debug, SurrealValue)]
    struct NarratesRow {
        hadith: surrealdb::types::RecordId,
        name_ar: Option<String>,
        name_en: String,
    }

    let hids: Vec<surrealdb::types::RecordId> =
        hadith_sources.iter().filter_map(|h| h.id.clone()).collect();

    let chain_map: std::collections::HashMap<String, Vec<String>> = if !hids.is_empty() {
        match db
            .query(
                "SELECT out AS hadith, in.name_ar AS name_ar, in.name_en AS name_en \
                 FROM narrates WHERE out IN $hids",
            )
            .bind(("hids", hids))
            .await
        {
            Ok(mut res) => {
                let rows: Vec<NarratesRow> = res.take(0).unwrap_or_default();
                let mut map: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();
                for row in rows {
                    let hkey = record_id_string(&row.hadith);
                    let name = row.name_ar.unwrap_or(row.name_en);
                    map.entry(hkey).or_default().push(name);
                }
                map
            }
            Err(e) => {
                tracing::error!("Batch narrator chain query failed: {e}");
                std::collections::HashMap::new()
            }
        }
    } else {
        std::collections::HashMap::new()
    };

    let mut context = String::from("## Relevant Hadiths:\n\n");
    for h in &hadith_sources {
        let narrator_text = h.narrator_text.as_deref().unwrap_or("Unknown narrator");

        let chain_str =
            h.id.as_ref()
                .and_then(|hid| chain_map.get(&record_id_string(hid)))
                .filter(|names| !names.is_empty())
                .map(|names| format!("Chain of narration: {}", names.join(" → ")))
                .unwrap_or_default();

        context.push_str(&format!(
            "Hadith #{} — {}\n",
            h.hadith_number, narrator_text
        ));
        if !chain_str.is_empty() {
            context.push_str(&format!("{chain_str}\n"));
        }
        context.push_str(&format!(
            "{}\n\n",
            h.text_en.as_deref().or(h.text_ar.as_deref()).unwrap_or("")
        ));
    }

    (hadith_sources, context)
}

/// Retrieve semantically-similar ayahs and format them as an LLM context block
/// headed by `## Relevant Quranic Verses:`, with inline tafsir_en truncated to
/// `max_tafsir_chars`. Swallows retrieval errors.
async fn retrieve_and_build_ayah_context(
    db: &Surreal<Db>,
    embedder: &dyn EmbeddingProvider,
    question: &str,
    k: usize,
    max_tafsir_chars: usize,
) -> (Vec<AyahSearchResult>, String) {
    let ayah_sources = crate::quran::search::search_ayahs_semantic(db, embedder, question, k, 0)
        .await
        .unwrap_or_default();

    let mut context = String::from("## Relevant Quranic Verses:\n\n");
    for a in &ayah_sources {
        let name = surah_name(a.surah_number);
        let text_en = a.text_en.as_deref().unwrap_or("");

        context.push_str(&format!(
            "Surah {} ({}:{}): {}\nArabic: {}\n",
            name, a.surah_number, a.ayah_number, text_en, a.text_ar,
        ));

        if let Some(ref tafsir) = a.tafsir_en
            && !tafsir.is_empty()
        {
            let truncated = if tafsir.len() > max_tafsir_chars {
                &tafsir[..tafsir.floor_char_boundary(max_tafsir_chars)]
            } else {
                tafsir
            };
            context.push_str(&format!("Tafsir Ibn Kathir: {truncated}\n"));
        }
        context.push('\n');
    }

    (ayah_sources, context)
}

// ── Scoped ask functions (semantic fallback paths) ──

/// Hadith-only semantic RAG: retrieve hadiths, ground the LLM in them.
pub async fn ask_hadith_only(
    provider: &dyn LlmProvider,
    db: &Surreal<Db>,
    embedder: &dyn EmbeddingProvider,
    question: &str,
    opts: &ChatOptions,
) -> Result<(Vec<HadithSearchResult>, TokenStream)> {
    let (hadith_sources, context) =
        retrieve_and_build_hadith_context(db, embedder, question, CONTEXT_HADITH_COUNT_SOLO).await;

    let system_prompt = format!(
        "You are a knowledgeable Islamic scholar specializing in hadith.\n\
         Answer questions using ONLY the hadiths provided below as context.\n\
         Always cite the hadith number when referencing a hadith.\n\
         When relevant, mention the chain of narration (isnad) to support authenticity.\n\
         If the context doesn't contain relevant information, say so honestly.\n\
         Be concise and accurate.\n\n{context}"
    );

    let stream = provider.chat_stream(&system_prompt, question, opts).await?;
    Ok((hadith_sources, stream))
}

/// Quran-only semantic RAG: retrieve ayahs + their inline Ibn Kathir tafsir,
/// ground the LLM in them.
pub async fn ask_quran_only(
    provider: &dyn LlmProvider,
    db: &Surreal<Db>,
    embedder: &dyn EmbeddingProvider,
    question: &str,
    opts: &ChatOptions,
) -> Result<(Vec<AyahSearchResult>, TokenStream)> {
    let (ayah_sources, context) = retrieve_and_build_ayah_context(
        db,
        embedder,
        question,
        CONTEXT_AYAH_COUNT_SOLO,
        MAX_TAFSIR_CHARS_SOLO,
    )
    .await;

    let system_prompt = format!(
        "You are a knowledgeable Quran scholar. Answer the user's question using ONLY \
         the Quranic verses and their tafsir (commentary by Ibn Kathir) provided below.\n\
         Always cite verse references (surah:ayah) for every claim.\n\
         If the provided verses don't contain relevant information, say so honestly.\n\
         Be concise and accurate.\n\n{context}"
    );

    let stream = provider.chat_stream(&system_prompt, question, opts).await?;
    Ok((ayah_sources, stream))
}

/// Retrieve relevant ayahs and hadiths, then stream an LLM answer grounded in both.
pub async fn ask_unified(
    provider: &dyn LlmProvider,
    db: &Surreal<Db>,
    embedder: &dyn EmbeddingProvider,
    question: &str,
    opts: &ChatOptions,
) -> Result<(Vec<AyahSearchResult>, Vec<HadithSearchResult>, TokenStream)> {
    let (ayah_sources, q_ctx) = retrieve_and_build_ayah_context(
        db,
        embedder,
        question,
        CONTEXT_AYAH_COUNT,
        MAX_TAFSIR_CHARS,
    )
    .await;
    let (hadith_sources, h_ctx) =
        retrieve_and_build_hadith_context(db, embedder, question, CONTEXT_HADITH_COUNT).await;

    let system_prompt = format!(
        "You are a knowledgeable Islamic scholar. Answer the user's question using ONLY \
         the Quranic verses and hadiths provided below as context.\n\
         When citing the Quran, always reference the surah and ayah number (e.g., 2:177).\n\
         When citing a hadith, always reference the hadith number.\n\
         When relevant, mention the chain of narration (isnad) to support hadith authenticity.\n\
         Draw from BOTH the Quran and the Sunnah (Prophetic tradition) when possible.\n\
         If the context doesn't contain relevant information, say so honestly.\n\
         Be concise and accurate.\n\n{q_ctx}{h_ctx}"
    );

    let stream = provider.chat_stream(&system_prompt, question, opts).await?;
    Ok((ayah_sources, hadith_sources, stream))
}
