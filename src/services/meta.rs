//! Meta / introspection services — corpus stats and runtime capability flags.

use anyhow::{Context, Result};
use serde::Serialize;
use surrealdb::types::SurrealValue;

use crate::models::StatsResponse;
use crate::web::AppState;

/// Capability flags reflecting which optional providers (LLM, embedder,
/// reranker) are wired up at runtime. Used by clients to decide whether to
/// surface Ask / semantic-search UI.
#[derive(Debug, Serialize)]
pub struct AppConfig {
    pub advanced_enabled: bool,
    pub llm_available: bool,
    pub llm_provider: Option<String>,
    pub embed_available: bool,
    pub embed_provider: Option<String>,
    pub reranker_available: bool,
}

pub fn app_config(state: &AppState) -> AppConfig {
    AppConfig {
        advanced_enabled: state.advanced_enabled,
        llm_available: state.llm.is_some(),
        llm_provider: state.llm.as_ref().map(|l| l.provider_name().to_string()),
        embed_available: state.embedder.is_some(),
        embed_provider: state
            .embedder
            .as_ref()
            .map(|e| e.provider_name().to_string()),
        reranker_available: state.reranker.is_some(),
    }
}

#[derive(Debug, SurrealValue)]
struct CountRow {
    c: i64,
}

/// Corpus row counts: hadith, narrator, collection.
pub async fn stats(state: &AppState) -> Result<StatsResponse> {
    let mut res = state
        .db
        .query(
            "SELECT count() AS c FROM hadith GROUP ALL; \
             SELECT count() AS c FROM narrator GROUP ALL; \
             SELECT count() AS c FROM collection GROUP ALL;",
        )
        .await
        .context("stats query failed")?;
    let take = |row: Option<CountRow>| row.map(|r| r.c).unwrap_or(0);
    Ok(StatsResponse {
        hadith_count: take(res.take(0).unwrap_or(None)),
        narrator_count: take(res.take(1).unwrap_or(None)),
        book_count: take(res.take(2).unwrap_or(None)),
    })
}
