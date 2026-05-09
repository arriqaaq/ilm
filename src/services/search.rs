//! Cross-corpus and per-corpus search services.
//!
//! - `search_hadith` / `search_quran` dispatch text / semantic / hybrid search
//!   modes over the underlying `crate::search` and `crate::quran::search`
//!   primitives.
//! - `search_unified` / `search_unified_text_only` interleave Quran + Hadith
//!   results via cross-source Reciprocal Rank Fusion.

use anyhow::{Result, anyhow};
use serde::Serialize;
use surrealdb::Surreal;

use crate::db::Db;
use crate::embed::RerankerBackend;
use crate::embedding::EmbeddingProvider;
use crate::models::{ApiHadithSearchResult, ApiNarratorSearchResult, HadithSearchResult};
use crate::quran::models::{ApiAyahSearchResult, AyahSearchResult};

/// Search modes accepted by `search_hadith` / `search_quran`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Text,
    Semantic,
    Hybrid,
}

impl SearchMode {
    /// Parse a user-supplied string. Defaults to `Hybrid` when input is
    /// `None` / empty.
    pub fn parse(s: Option<&str>) -> Result<Self> {
        match s.unwrap_or("hybrid").to_ascii_lowercase().as_str() {
            "text" => Ok(SearchMode::Text),
            "semantic" => Ok(SearchMode::Semantic),
            "hybrid" | "" => Ok(SearchMode::Hybrid),
            other => Err(anyhow!("unknown search mode: {other}")),
        }
    }
}

/// Hadith corpus search dispatcher (text / semantic / hybrid).
///
/// Semantic and hybrid modes require an embedder; text mode does not.
/// `rerank` only takes effect for hybrid mode and only when a reranker is
/// configured.
pub async fn search_hadith(
    db: &Surreal<Db>,
    embedder: Option<&dyn EmbeddingProvider>,
    reranker: Option<&RerankerBackend>,
    query: &str,
    mode: SearchMode,
    limit: usize,
    offset: usize,
    rerank: bool,
) -> Result<Vec<HadithSearchResult>> {
    match (mode, embedder) {
        (SearchMode::Text, _) => crate::search::search_hadiths_text(db, query, limit, offset).await,
        (SearchMode::Semantic, Some(e)) => {
            crate::search::search_hadiths_semantic(db, e, query, limit).await
        }
        (SearchMode::Hybrid, Some(e)) => {
            crate::search::search_hadiths_hybrid(
                db,
                e,
                query,
                limit,
                offset,
                if rerank { reranker } else { None },
            )
            .await
        }
        (SearchMode::Semantic | SearchMode::Hybrid, None) => Err(anyhow!(
            "search mode requires an embedder; server started in lite mode"
        )),
    }
}

/// Combined hadith + narrator search response.
#[derive(Debug, Serialize)]
pub struct HadithAndNarratorResults {
    pub query: String,
    pub search_type: String,
    pub hadiths: Vec<ApiHadithSearchResult>,
    pub narrators: Vec<ApiNarratorSearchResult>,
}

/// Combined `/search/hadith` shape: dispatch hadith search + narrator search,
/// merge into a single response. Empty query returns empty arrays.
pub async fn search_hadith_and_narrators(
    state: &crate::web::AppState,
    query: &str,
    mode: SearchMode,
    limit: usize,
    rerank: bool,
) -> Result<HadithAndNarratorResults> {
    let mode_str = match mode {
        SearchMode::Text => "text",
        SearchMode::Semantic => "semantic",
        SearchMode::Hybrid => "hybrid",
    };
    let query_str = query.to_string();
    if query.is_empty() {
        return Ok(HadithAndNarratorResults {
            query: query_str,
            search_type: mode_str.to_string(),
            hadiths: vec![],
            narrators: vec![],
        });
    }

    let embedder = state.embedder.as_deref();

    // Hadith search: degrade to text when embedder is missing in semantic/hybrid modes.
    let hadiths_raw = match (mode, embedder) {
        (SearchMode::Text, _) => crate::search::search_hadiths_text(&state.db, query, limit, 0)
            .await
            .unwrap_or_default(),
        (SearchMode::Semantic, Some(e)) => {
            crate::search::search_hadiths_semantic(&state.db, e, query, limit)
                .await
                .unwrap_or_default()
        }
        (SearchMode::Hybrid, Some(e)) => {
            let r = if rerank {
                state.reranker.as_deref()
            } else {
                None
            };
            crate::search::search_hadiths_hybrid(&state.db, e, query, limit, 0, r)
                .await
                .unwrap_or_default()
        }
        (SearchMode::Semantic | SearchMode::Hybrid, None) => {
            crate::search::search_hadiths_text(&state.db, query, limit, 0)
                .await
                .unwrap_or_default()
        }
    };

    let narrators_raw = crate::search::search_narrators(&state.db, query, 10, 0)
        .await
        .unwrap_or_default();

    Ok(HadithAndNarratorResults {
        query: query_str,
        search_type: mode_str.to_string(),
        hadiths: hadiths_raw
            .into_iter()
            .map(ApiHadithSearchResult::from)
            .collect(),
        narrators: narrators_raw
            .into_iter()
            .map(ApiNarratorSearchResult::from)
            .collect(),
    })
}

/// Quran corpus search dispatcher (text / semantic / hybrid).
pub async fn search_quran(
    db: &Surreal<Db>,
    embedder: Option<&dyn EmbeddingProvider>,
    query: &str,
    mode: SearchMode,
    limit: usize,
    offset: usize,
) -> Result<Vec<AyahSearchResult>> {
    match (mode, embedder) {
        (SearchMode::Text, _) => {
            crate::quran::search::search_ayahs_text(db, query, limit, offset).await
        }
        (SearchMode::Semantic, Some(e)) => {
            crate::quran::search::search_ayahs_semantic(db, e, query, limit, offset).await
        }
        (SearchMode::Hybrid, Some(e)) => {
            crate::quran::search::search_ayahs_hybrid(db, e, query, limit, offset).await
        }
        (SearchMode::Semantic | SearchMode::Hybrid, None) => Err(anyhow!(
            "search mode requires an embedder; server started in lite mode"
        )),
    }
}

/// A single item in the unified search results — either a Quran ayah or a Hadith.
#[derive(Debug, Serialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum UnifiedSearchItem {
    Quran {
        #[serde(flatten)]
        ayah: ApiAyahSearchResult,
        unified_score: f64,
    },
    Hadith {
        #[serde(flatten)]
        hadith: ApiHadithSearchResult,
        unified_score: f64,
    },
}

/// Response from the unified search endpoint.
#[derive(Debug, Serialize)]
pub struct UnifiedSearchResponse {
    pub query: String,
    pub search_type: String,
    pub results: Vec<UnifiedSearchItem>,
    pub quran_count: usize,
    pub hadith_count: usize,
    pub page: usize,
    pub has_more: bool,
}

/// Reciprocal Rank Fusion score: 1 / (k + rank), with k = 60.
fn rrf_score(rank: usize) -> f64 {
    1.0 / (60.0 + rank as f64)
}

/// Search both Quran ayahs and Hadiths, then interleave via cross-source RRF with pagination.
pub async fn search_unified(
    db: &Surreal<Db>,
    embedder: &dyn EmbeddingProvider,
    query: &str,
    search_type: &str,
    limit: usize,
    page: usize,
    reranker: Option<&RerankerBackend>,
) -> Result<UnifiedSearchResponse> {
    // Fetch enough from each source to fill this page + detect has_more.
    // We need (page * limit) items total from the merged list, plus 1 to check has_more.
    let fetch_per_source = page * limit + 1;

    tracing::debug!(
        "unified search: query={query:?} type={search_type} limit={limit} page={page} fetch_per_source={fetch_per_source}"
    );

    // Run searches sequentially to avoid doubling HNSW stack usage on one worker thread
    let hadiths = match search_type {
        "semantic" => crate::search::search_hadiths_semantic(db, embedder, query, fetch_per_source)
            .await
            .unwrap_or_default(),
        "text" => crate::search::search_hadiths_text(db, query, fetch_per_source, 0)
            .await
            .unwrap_or_default(),
        _ => {
            crate::search::search_hadiths_hybrid(db, embedder, query, fetch_per_source, 0, reranker)
                .await
                .unwrap_or_default()
        }
    };

    let ayahs = match search_type {
        "semantic" => {
            crate::quran::search::search_ayahs_semantic(db, embedder, query, fetch_per_source, 0)
                .await
                .unwrap_or_default()
        }
        "text" => crate::quran::search::search_ayahs_text(db, query, fetch_per_source, 0)
            .await
            .unwrap_or_default(),
        _ => crate::quran::search::search_ayahs_hybrid(db, embedder, query, fetch_per_source, 0)
            .await
            .unwrap_or_default(),
    };

    let quran_count = ayahs.len();
    let hadith_count = hadiths.len();

    // Cross-source RRF: assign ranks within each list, compute unified scores, merge
    let mut items: Vec<UnifiedSearchItem> = Vec::with_capacity(quran_count + hadith_count);

    for (rank, ayah) in ayahs.into_iter().enumerate() {
        items.push(UnifiedSearchItem::Quran {
            ayah: ApiAyahSearchResult::from(ayah),
            unified_score: rrf_score(rank + 1),
        });
    }

    for (rank, hadith) in hadiths.into_iter().enumerate() {
        items.push(UnifiedSearchItem::Hadith {
            hadith: ApiHadithSearchResult::from(hadith),
            unified_score: rrf_score(rank + 1),
        });
    }

    // Sort by unified_score descending (interleaves the two sources)
    items.sort_by(|a, b| {
        let sa = match a {
            UnifiedSearchItem::Quran { unified_score, .. } => *unified_score,
            UnifiedSearchItem::Hadith { unified_score, .. } => *unified_score,
        };
        let sb = match b {
            UnifiedSearchItem::Quran { unified_score, .. } => *unified_score,
            UnifiedSearchItem::Hadith { unified_score, .. } => *unified_score,
        };
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Paginate: skip to the right page, take limit items
    let offset = (page - 1) * limit;
    let has_more = items.len() > offset + limit;
    let results: Vec<UnifiedSearchItem> = items.into_iter().skip(offset).take(limit).collect();

    Ok(UnifiedSearchResponse {
        query: query.to_string(),
        search_type: search_type.to_string(),
        results,
        quran_count,
        hadith_count,
        page,
        has_more,
    })
}

/// Text-only unified search (no embedder required). Used when advanced features are disabled.
pub async fn search_unified_text_only(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
    page: usize,
) -> Result<UnifiedSearchResponse> {
    let fetch_per_source = page * limit + 1;

    let hadiths = crate::search::search_hadiths_text(db, query, fetch_per_source, 0)
        .await
        .unwrap_or_default();

    let ayahs = crate::quran::search::search_ayahs_text(db, query, fetch_per_source, 0)
        .await
        .unwrap_or_default();

    let quran_count = ayahs.len();
    let hadith_count = hadiths.len();

    let mut items: Vec<UnifiedSearchItem> = Vec::with_capacity(quran_count + hadith_count);

    for (rank, ayah) in ayahs.into_iter().enumerate() {
        items.push(UnifiedSearchItem::Quran {
            ayah: ApiAyahSearchResult::from(ayah),
            unified_score: rrf_score(rank + 1),
        });
    }

    for (rank, hadith) in hadiths.into_iter().enumerate() {
        items.push(UnifiedSearchItem::Hadith {
            hadith: ApiHadithSearchResult::from(hadith),
            unified_score: rrf_score(rank + 1),
        });
    }

    items.sort_by(|a, b| {
        let sa = match a {
            UnifiedSearchItem::Quran { unified_score, .. } => *unified_score,
            UnifiedSearchItem::Hadith { unified_score, .. } => *unified_score,
        };
        let sb = match b {
            UnifiedSearchItem::Quran { unified_score, .. } => *unified_score,
            UnifiedSearchItem::Hadith { unified_score, .. } => *unified_score,
        };
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let offset = (page - 1) * limit;
    let has_more = items.len() > offset + limit;
    let results: Vec<UnifiedSearchItem> = items.into_iter().skip(offset).take(limit).collect();

    Ok(UnifiedSearchResponse {
        query: query.to_string(),
        search_type: "text".to_string(),
        results,
        quran_count,
        hadith_count,
        page,
        has_more,
    })
}
