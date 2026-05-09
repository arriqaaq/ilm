//! MCP tool methods.
//!
//! Every tool is a 3–6 line wrapper that calls into `crate::services` and
//! serializes the result via `Content::json`. The single `#[tool_router]`
//! impl block on `McpServer` lives here so all tools share one
//! `ToolRouter<Self>` registration.
//!
//! Phase 2 ships the tools whose underlying services already exist after the
//! relocation (search, ask, narrator, classify). Phase 3 adds the remaining
//! tools as their corresponding HTTP handlers are extracted into services.

use futures::StreamExt;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_handler, tool_router};
use serde::Deserialize;

use crate::mcp::{McpServer, mcp_err};
use crate::models::NoteRef;
use crate::services;
use crate::web::AppState;

impl McpServer {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "Quran & Hadith corpus tools: search/lookup over verses, hadiths, \
                 narrators, isnad chains, tafsir, mustalah analysis, and grading."
                    .to_string(),
            )
    }
}

// ── Input arg types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchArgs {
    /// Search query (Arabic, English, or transliteration).
    pub query: String,
    /// Search mode: "text" (BM25), "semantic" (vector), or "hybrid" (RRF). Default: hybrid.
    #[serde(default)]
    pub mode: Option<String>,
    /// Max results (1..=100, default 20).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Result offset for pagination (default 0).
    #[serde(default)]
    pub offset: Option<u32>,
    /// Re-rank top candidates with cross-encoder/LLM (hybrid mode only, default false).
    #[serde(default)]
    pub rerank: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UnifiedSearchArgs {
    /// Search query.
    pub query: String,
    /// "text" | "semantic" | "hybrid" (default: hybrid).
    #[serde(default)]
    pub mode: Option<String>,
    /// Page size (default 20).
    #[serde(default)]
    pub limit: Option<u32>,
    /// 1-indexed page (default 1).
    #[serde(default)]
    pub page: Option<u32>,
    /// Re-rank hybrid candidates (default false).
    #[serde(default)]
    pub rerank: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AskArgs {
    /// The question to answer.
    pub question: String,
    /// Optional model override (defaults to server-configured model).
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NarratorByNameArgs {
    /// Narrator name (Arabic, English, kunya, or alias). Fuzzy-matched.
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NarratorCountArgs {
    pub name: String,
    /// Optional book name filter (e.g. "Bukhari").
    #[serde(default)]
    pub book: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClassifyArgs {
    /// Free-form question to classify into a `QueryIntent`.
    pub question: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetByIdArgs {
    /// Resource id (e.g. "bukhari:1" for hadith, "narrator:abu-hurayra").
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListHadithsArgs {
    /// Filter by `collection_id` (1=Bukhari, 2=Muslim, 3=Abu Dawud, 4=Tirmidhi, 5=Nasai, 6=Ibn Majah).
    #[serde(default)]
    pub book: Option<i64>,
    /// Filter by `hadith_number` within the book.
    #[serde(default)]
    pub number: Option<i64>,
    /// 1-indexed page (default 1).
    #[serde(default)]
    pub page: Option<u32>,
    /// Page size (1..=100, default 20).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListNarratorsArgs {
    /// Free-text name filter (matches name_en case-insensitive OR name_ar substring).
    #[serde(default)]
    pub q: Option<String>,
    /// Filter by narrator generation (tabaqah).
    #[serde(default)]
    pub generation: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AutocompleteArgs {
    /// Prefix or substring to match.
    pub q: String,
    /// Max suggestions (1..=50, default 8).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PageLimitArgs {
    /// 1-indexed page (default 1).
    #[serde(default)]
    pub page: Option<u32>,
    /// Page size (1..=100, default 20).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SurahArgs {
    /// Surah number 1..=114.
    pub number: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AyahArgs {
    /// Surah number 1..=114.
    pub surah: i64,
    /// Ayah number within the surah.
    pub ayah: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AyahHadithsArgs {
    pub surah: i64,
    pub ayah: i64,
    /// Include semantically-related hadiths in addition to curated ones (default false).
    #[serde(default)]
    pub include_semantic: Option<bool>,
    /// Max semantic results (default 5).
    #[serde(default)]
    pub semantic_limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowseAyahsArgs {
    /// Optional surah filter (1..=114). If omitted, paginates the whole Quran.
    #[serde(default)]
    pub surah: Option<i64>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RootArgs {
    /// Arabic root (e.g. "ك ت ب").
    pub root: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListBooksArgs {
    /// Filter by category: tafsir, hadith_sharh, hadith_grading, hadith_collection, biography.
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBookArgs {
    pub book_id: u64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBookPagesArgs {
    pub book_id: u64,
    /// 0-indexed page offset (default 0).
    #[serde(default)]
    pub start: Option<u64>,
    /// Page size, capped at 100 (default 20).
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AyahTafsirArgs {
    pub surah: u64,
    pub ayah: u64,
    /// Tafsir book id. Defaults to Ibn Kathir.
    #[serde(default)]
    pub book_id: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SurahTafsirPagesArgs {
    pub surah_number: u64,
    /// Tafsir book id. Defaults to Ibn Kathir.
    #[serde(default)]
    pub book_id: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CompareMatnArgs {
    /// First hadith id (e.g. "bukhari:1").
    pub a: String,
    /// Second hadith id (e.g. "muslim:5").
    pub b: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IsnadSearchArgs {
    /// At least 2 narrator slugs that must all appear in the chain.
    pub narrator_ids: Vec<String>,
    /// "loose" (default) — any order, any positions. "strict" — must form a
    /// contiguous student → teacher sub-chain in the order provided.
    #[serde(default)]
    pub mode: Option<String>,
    /// Max hadiths returned (1..=100, default 20).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CommonNarratorsArgs {
    /// First narrator slug.
    pub a: String,
    /// Second narrator slug.
    pub b: String,
}

// ── Note/notebook arg structs ──────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListNotesArgs {
    #[serde(default)]
    pub ref_type: Option<String>,
    #[serde(default)]
    pub ref_id: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    /// Notebook id filter; pass "__uncategorized__" to get notes with no notebook.
    #[serde(default)]
    pub notebook_id: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateNoteArgs {
    /// What kind of thing this note anchors to: "hadith" | "ayah" | "narrator" | "topic" | etc.
    pub ref_type: String,
    /// The id of the anchored entity (omit for free-floating "topic" notes).
    #[serde(default)]
    pub ref_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    /// Sticky-note color: "yellow" (default), "pink", "blue", "green", "purple".
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Additional refs to attach (multi-anchor notes).
    #[serde(default)]
    pub refs: Option<Vec<NoteRef>>,
    #[serde(default)]
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateNoteArgs {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub refs: Option<Vec<NoteRef>>,
    #[serde(default)]
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulkRefsArgs {
    pub ref_type: String,
    pub ref_ids: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateNoteRefsArgs {
    pub id: String,
    /// "add" or "remove".
    pub action: String,
    #[serde(rename = "ref")]
    pub note_ref: NoteRef,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateRefAnnotationArgs {
    pub id: String,
    /// 0-indexed position of the ref in the note's refs array.
    pub idx: u32,
    pub annotation: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateNotebookArgs {
    pub name: String,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateNotebookArgs {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub sort_order: Option<i32>,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn clamp_limit(limit: Option<u32>) -> usize {
    limit.unwrap_or(20).clamp(1, 100) as usize
}

fn require_llm<'a>(
    state: &'a crate::web::AppState,
) -> Result<&'a dyn crate::llm::LlmProvider, McpError> {
    state
        .llm
        .as_deref()
        .ok_or_else(|| McpError::invalid_request("LLM not configured (lite mode)", None))
}

fn require_embedder<'a>(
    state: &'a crate::web::AppState,
) -> Result<&'a dyn crate::embedding::EmbeddingProvider, McpError> {
    state
        .embedder
        .as_deref()
        .ok_or_else(|| McpError::invalid_request("Embedder not configured (lite mode)", None))
}

/// Map service-side note errors to MCP errors. Forwards "not found" /
/// "bad_request" markers to `invalid_request`; everything else is `internal`.
fn note_err(e: anyhow::Error) -> McpError {
    if services::notes::is_not_found(&e) || services::notes::is_bad_request(&e) {
        McpError::invalid_request(e.to_string(), None)
    } else {
        mcp_err(e)
    }
}

async fn drain_stream(mut stream: crate::llm::TokenStream) -> Result<String, McpError> {
    let mut answer = String::new();
    while let Some(item) = stream.next().await {
        let ev = item.map_err(mcp_err)?;
        answer.push_str(&ev.delta);
        if ev.done {
            break;
        }
    }
    Ok(answer)
}

// ── Tools ───────────────────────────────────────────────────────────────────

#[tool_router]
impl McpServer {
    // ── Search ─────────────────────────────────────────────────────────────

    #[tool(
        description = "Search the hadith corpus (BM25 text / semantic vector / hybrid RRF). \
                       Returns ranked HadithSearchResult JSON list with score, text_ar, text_en, narrator_text."
    )]
    async fn search_hadith(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mode = services::search::SearchMode::parse(args.mode.as_deref()).map_err(mcp_err)?;
        let limit = clamp_limit(args.limit);
        let offset = args.offset.unwrap_or(0) as usize;
        let results = services::search::search_hadith(
            &self.state.db,
            self.state.embedder.as_deref(),
            self.state.reranker.as_deref(),
            &args.query,
            mode,
            limit,
            offset,
            args.rerank.unwrap_or(false),
        )
        .await
        .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(
            serde_json::json!({
                "query": args.query,
                "mode": format!("{mode:?}").to_lowercase(),
                "results": results,
            }),
        )?]))
    }

    #[tool(
        description = "Search the Quran corpus (BM25 text / semantic vector / hybrid RRF). \
                       Returns ranked AyahSearchResult list with surah_number, ayah_number, text_ar, text_en, tafsir_en."
    )]
    async fn search_quran(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mode = services::search::SearchMode::parse(args.mode.as_deref()).map_err(mcp_err)?;
        let limit = clamp_limit(args.limit);
        let offset = args.offset.unwrap_or(0) as usize;
        let results = services::search::search_quran(
            &self.state.db,
            self.state.embedder.as_deref(),
            &args.query,
            mode,
            limit,
            offset,
        )
        .await
        .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(
            serde_json::json!({
                "query": args.query,
                "mode": format!("{mode:?}").to_lowercase(),
                "results": results,
            }),
        )?]))
    }

    #[tool(
        description = "Cross-corpus unified search interleaving Quran ayahs and hadiths via \
                       reciprocal rank fusion. Each result has `source: \"quran\" | \"hadith\"`."
    )]
    async fn search_unified(
        &self,
        Parameters(args): Parameters<UnifiedSearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mode = services::search::SearchMode::parse(args.mode.as_deref()).map_err(mcp_err)?;
        let mode_str = match mode {
            services::search::SearchMode::Text => "text",
            services::search::SearchMode::Semantic => "semantic",
            services::search::SearchMode::Hybrid => "hybrid",
        };
        let limit = clamp_limit(args.limit);
        let page = args.page.unwrap_or(1).max(1) as usize;

        let response = match self.state.embedder.as_deref() {
            Some(e) => services::search::search_unified(
                &self.state.db,
                e,
                &args.query,
                mode_str,
                limit,
                page,
                if args.rerank.unwrap_or(false) {
                    self.state.reranker.as_deref()
                } else {
                    None
                },
            )
            .await
            .map_err(mcp_err)?,
            None => {
                services::search::search_unified_text_only(&self.state.db, &args.query, limit, page)
                    .await
                    .map_err(mcp_err)?
            }
        };

        Ok(CallToolResult::success(vec![Content::json(response)?]))
    }

    // ── Hadith retrieval ───────────────────────────────────────────────────

    #[tool(
        description = "List the six hadith collections (Bukhari, Muslim, Abu Dawud, Tirmidhi, \
                       Nasai, Ibn Majah) ordered by collection_id."
    )]
    async fn list_collections(&self) -> Result<CallToolResult, McpError> {
        let collections = services::hadith::list_collections(&self.state)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(collections)?]))
    }

    #[tool(
        description = "Paginated hadith list with optional book / hadith_number filters. \
                       Returns { data: [ApiHadith...], page, limit, has_more }."
    )]
    async fn list_hadiths(
        &self,
        Parameters(args): Parameters<ListHadithsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page = args.page.unwrap_or(1).max(1) as usize;
        let limit = clamp_limit(args.limit);
        let resp = services::hadith::list(&self.state, args.book, args.number, page, limit)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Fetch a single hadith by id (e.g. \"bukhari:1\") with its transmission \
                       chain (narrators), linked Quran ayahs, and semantically-similar hadiths."
    )]
    async fn get_hadith(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let detail = services::hadith::get_detail(&self.state, &args.id)
            .await
            .map_err(|e| {
                if services::hadith::is_not_found(&e) {
                    McpError::invalid_request(e.to_string(), None)
                } else {
                    mcp_err(e)
                }
            })?;
        Ok(CallToolResult::success(vec![Content::json(detail)?]))
    }

    // ── Narrator browse / detail ───────────────────────────────────────────

    #[tool(
        description = "Paginated narrator list, sorted by hadith_count DESC. Optional free-text \
                       (`q`) and generation/tabaqah filter. Returns { data: [ApiNarratorWithCount...], page, limit, has_more }."
    )]
    async fn list_narrators(
        &self,
        Parameters(args): Parameters<ListNarratorsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page = args.page.unwrap_or(1).max(1) as usize;
        let limit = clamp_limit(args.limit);
        let resp = services::narrator::list(
            &self.state,
            args.q.as_deref(),
            args.generation.as_deref(),
            page,
            limit,
        )
        .await
        .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Fetch a narrator by id (the slug part, e.g. \"abu-hurayra\") with their \
                       biographical fields, sample hadiths, deduplicated teachers, and deduplicated students."
    )]
    async fn get_narrator(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let detail = services::narrator::get_detail(&self.state, &args.id)
            .await
            .map_err(|e| {
                if services::narrator::is_not_found(&e) {
                    McpError::invalid_request(e.to_string(), None)
                } else {
                    mcp_err(e)
                }
            })?;
        Ok(CallToolResult::success(vec![Content::json(detail)?]))
    }

    #[tool(
        description = "Typeahead autocomplete over narrator names (English, Arabic, kunya, \
                       aliases, search_name slug). Sorted by hadith_count."
    )]
    async fn narrator_autocomplete(
        &self,
        Parameters(args): Parameters<AutocompleteArgs>,
    ) -> Result<CallToolResult, McpError> {
        let limit = args.limit.unwrap_or(8).clamp(1, 50) as usize;
        let rows = services::narrator::autocomplete(&self.state, &args.q, limit)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(rows)?]))
    }

    // ── Quran ──────────────────────────────────────────────────────────────

    #[tool(description = "Aggregate Quran corpus counts: { surah_count, ayah_count }.")]
    async fn get_quran_stats(&self) -> Result<CallToolResult, McpError> {
        let stats = services::quran::stats(&self.state).await.map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(stats)?]))
    }

    #[tool(description = "All 114 surahs with metadata (name, ayah_count, revelation_type).")]
    async fn list_surahs(&self) -> Result<CallToolResult, McpError> {
        let s = services::quran::list_surahs(&self.state)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(s)?]))
    }

    #[tool(
        description = "One surah plus all its ayahs (Arabic text, English translation, juz/hizb)."
    )]
    async fn get_surah(
        &self,
        Parameters(args): Parameters<SurahArgs>,
    ) -> Result<CallToolResult, McpError> {
        let resp = services::quran::get_surah(&self.state, args.number)
            .await
            .map_err(|e| {
                if services::quran::is_not_found(&e) {
                    McpError::invalid_request(e.to_string(), None)
                } else {
                    mcp_err(e)
                }
            })?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Browse ayahs (paginated). Optional `surah` filter restricts to one surah."
    )]
    async fn browse_ayahs(
        &self,
        Parameters(args): Parameters<BrowseAyahsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page = args.page.unwrap_or(1).max(1) as usize;
        let limit = args.limit.unwrap_or(50).clamp(1, 200) as usize;
        let resp = services::quran::browse_ayahs(&self.state, args.surah, page, limit)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Word-by-word morphology for one ayah: each word's text_ar, root, lemma, \
                       POS, transliteration, English gloss. Sourced from corpus.quran.com + QUL."
    )]
    async fn get_ayah_words(
        &self,
        Parameters(args): Parameters<AyahArgs>,
    ) -> Result<CallToolResult, McpError> {
        let words = services::quran::get_ayah_words(&self.state, args.surah, args.ayah)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(words)?]))
    }

    #[tool(
        description = "Hadiths linked to a Quran ayah. `curated` is always returned (curated \
                       references_hadith edges from Quran.com); set `include_semantic=true` to \
                       also include semantically-related hadiths via vector search."
    )]
    async fn get_ayah_hadiths(
        &self,
        Parameters(args): Parameters<AyahHadithsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let semantic = args.include_semantic.unwrap_or(false);
        let lim = args.semantic_limit.unwrap_or(5).clamp(1, 50) as usize;
        let resp =
            services::quran::get_ayah_hadiths(&self.state, args.surah, args.ayah, semantic, lim)
                .await
                .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Per-ayah curated hadith counts for one surah. Map key = ayah_number string."
    )]
    async fn get_surah_hadith_counts(
        &self,
        Parameters(args): Parameters<SurahArgs>,
    ) -> Result<CallToolResult, McpError> {
        let counts = services::quran::get_surah_hadith_counts(&self.state, args.number)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(counts)?]))
    }

    #[tool(
        description = "Per-ayah counts of similar_to + shares_phrase edges (mutashabihat markers)."
    )]
    async fn get_surah_similar_counts(
        &self,
        Parameters(args): Parameters<SurahArgs>,
    ) -> Result<CallToolResult, McpError> {
        let counts = services::quran::get_surah_similar_counts(&self.state, args.number)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(counts)?]))
    }

    #[tool(
        description = "Concordance of every Quran occurrence of an Arabic root. Returns \
                       { root, occurrences: [ApiQuranWord...], ayah_count }."
    )]
    async fn search_quran_root(
        &self,
        Parameters(args): Parameters<RootArgs>,
    ) -> Result<CallToolResult, McpError> {
        let resp = services::quran::search_by_root(&self.state, &args.root)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(description = "All reciters available for ayah-level audio playback.")]
    async fn list_reciters(&self) -> Result<CallToolResult, McpError> {
        let r = services::quran::list_reciters(&self.state)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(r)?]))
    }

    // ── Combined search + isnad / common narrator analysis ────────────────

    #[tool(
        description = "Search hadiths AND narrators in a single call. Same modes as search_hadith \
                       (text/semantic/hybrid). Returns { query, search_type, hadiths: [...], narrators: [...] }."
    )]
    async fn search_hadith_and_narrators(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mode = services::search::SearchMode::parse(args.mode.as_deref()).map_err(mcp_err)?;
        let limit = clamp_limit(args.limit);
        let resp = services::search::search_hadith_and_narrators(
            &self.state,
            &args.query,
            mode,
            limit,
            args.rerank.unwrap_or(false),
        )
        .await
        .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Find hadiths whose isnad chain contains every supplied narrator slug. \
                       mode=\"loose\" (default) — any order. mode=\"strict\" — narrators must \
                       form a contiguous student→teacher sub-chain in the order given."
    )]
    async fn search_isnad(
        &self,
        Parameters(args): Parameters<IsnadSearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let limit = clamp_limit(args.limit);
        let mode = args.mode.as_deref().unwrap_or("loose");
        let resp = services::hadith::isnad_search(&self.state, &args.narrator_ids, mode, limit)
            .await
            .map_err(|e| {
                if services::hadith::is_not_found(&e) || services::hadith::is_bad_request(&e) {
                    McpError::invalid_request(e.to_string(), None)
                } else {
                    mcp_err(e)
                }
            })?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Given two narrator slugs `a` and `b`, return both anchors plus the (up to 50) \
                       narrators who also appear in chains of hadiths shared by both. Useful for \
                       discovering madar al-isnad / shared transmitters between two scholars."
    )]
    async fn list_common_narrators(
        &self,
        Parameters(args): Parameters<CommonNarratorsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let resp = services::narrator::list_common(&self.state, &args.a, &args.b)
            .await
            .map_err(|e| {
                if services::narrator::is_not_found(&e) {
                    McpError::invalid_request(e.to_string(), None)
                } else {
                    mcp_err(e)
                }
            })?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    // ── Hadith / narrator graphs + matn diff ───────────────────────────────

    #[tool(
        description = "Cytoscape-shaped isnad graph for one hadith (e.g. \"bukhari:1\"). \
                       Nodes are narrators in the chain, edges are heard_from links labeled \
                       with chain_position."
    )]
    async fn get_chain_graph(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let g = services::hadith::get_chain_graph(&self.state, &args.id)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(g)?]))
    }

    #[tool(
        description = "Cytoscape-shaped narrator network — the centre narrator plus their \
                       deduplicated immediate teachers (incoming) and students (outgoing). \
                       Capped at 25 nodes per side."
    )]
    async fn get_narrator_graph(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let g = services::narrator::get_graph(&self.state, &args.id)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(g)?]))
    }

    #[tool(
        description = "How often this narrator appears as a structural pivot or bottleneck \
                       across analyzed families. Pair with analyze_family_mustalah to reason \
                       about isnad reliability."
    )]
    async fn get_narrator_isnad_role(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let role = services::narrator::get_isnad_role(&self.state, &args.id)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(role)?]))
    }

    #[tool(
        description = "Word-level matn diff between two hadiths (a, b). Prefers the `matn` \
                       field (body without sanad) when present, falls back to text_ar / text_en. \
                       Use to study narrator paraphrases across variants of the same family."
    )]
    async fn compare_matn(
        &self,
        Parameters(args): Parameters<CompareMatnArgs>,
    ) -> Result<CallToolResult, McpError> {
        let result = services::hadith::matn_diff(&self.state, &args.a, &args.b)
            .await
            .map_err(|e| {
                if services::hadith::is_not_found(&e) {
                    McpError::invalid_request(e.to_string(), None)
                } else {
                    mcp_err(e)
                }
            })?;
        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    // ── Scholars / gradings ────────────────────────────────────────────────

    #[tool(
        description = "All scholars known to the grading dataset, with the count of distinct \
                       hadith verdicts each has authored."
    )]
    async fn list_scholars(&self) -> Result<CallToolResult, McpError> {
        let s = services::scholars::list_scholars(&self.state)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(s)?]))
    }

    #[tool(
        description = "Multi-scholar verdicts on one hadith (e.g. \"bukhari:1\"). For Bukhari/\
                       Muslim hadiths a synthetic 'consensus sahih' row is prepended."
    )]
    async fn get_hadith_gradings(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let resp = services::scholars::get_hadith_gradings(&self.state, &args.id)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    // ── Meta / introspection ───────────────────────────────────────────────

    #[tool(description = "Corpus row counts: { hadith_count, narrator_count, book_count }.")]
    async fn get_stats(&self) -> Result<CallToolResult, McpError> {
        let s = services::meta::stats(&self.state).await.map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(s)?]))
    }

    #[tool(
        description = "Runtime capability flags — which optional providers (LLM, embedder, \
                       reranker) are wired up. Use to decide which other tools will work."
    )]
    async fn get_app_config(&self) -> Result<CallToolResult, McpError> {
        let cfg = services::meta::app_config(&self.state);
        Ok(CallToolResult::success(vec![Content::json(cfg)?]))
    }

    // ── Books / tafsir ─────────────────────────────────────────────────────

    #[tool(
        description = "All ingested turath books, optionally filtered by category (tafsir, \
                       hadith_sharh, hadith_grading, hadith_collection, biography)."
    )]
    async fn list_books(
        &self,
        Parameters(args): Parameters<ListBooksArgs>,
    ) -> Result<CallToolResult, McpError> {
        let books = services::book::list(&self.state, args.category.as_deref())
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(books)?]))
    }

    #[tool(description = "One book's metadata + table-of-contents headings.")]
    async fn get_book(
        &self,
        Parameters(args): Parameters<GetBookArgs>,
    ) -> Result<CallToolResult, McpError> {
        let book = services::book::get(&self.state, args.book_id)
            .await
            .map_err(|e| {
                if services::book::is_not_found(&e) {
                    McpError::invalid_request(e.to_string(), None)
                } else {
                    mcp_err(e)
                }
            })?;
        Ok(CallToolResult::success(vec![Content::json(book)?]))
    }

    #[tool(
        description = "Page window for a book (start..start+size, ordered by page_index). \
                       Returns { pages: [{ page_index, text, vol, page_num }], total, start, size }."
    )]
    async fn get_book_pages(
        &self,
        Parameters(args): Parameters<GetBookPagesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let start = args.start.unwrap_or(0);
        let size = args.size.unwrap_or(20);
        let resp = services::book::get_pages(&self.state, args.book_id, start, size)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Tafsir body for one ayah from one tafsir book. Defaults to Ibn Kathir \
                       when book_id omitted. Returns full text + heading + page metadata."
    )]
    async fn get_ayah_tafsir(
        &self,
        Parameters(args): Parameters<AyahTafsirArgs>,
    ) -> Result<CallToolResult, McpError> {
        let book_id = args
            .book_id
            .unwrap_or(services::tafsir::DEFAULT_TAFSIR_BOOK_ID);
        let resp = services::tafsir::get_ayah_tafsir(&self.state, args.surah, args.ayah, book_id)
            .await
            .map_err(mcp_err)?
            .ok_or_else(|| {
                McpError::invalid_request(
                    format!(
                        "no tafsir mapping for {}:{} in book {}",
                        args.surah, args.ayah, book_id
                    ),
                    None,
                )
            })?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Per-ayah → tafsir-page index for one surah, for one tafsir book. Useful \
                       for rendering heading markers in the Quran reader."
    )]
    async fn get_surah_tafsir_pages(
        &self,
        Parameters(args): Parameters<SurahTafsirPagesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let book_id = args
            .book_id
            .unwrap_or(services::tafsir::DEFAULT_TAFSIR_BOOK_ID);
        let resp =
            services::tafsir::get_surah_tafsir_pages(&self.state, args.surah_number, book_id)
                .await
                .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    // ── Family / mustalah ──────────────────────────────────────────────────

    #[tool(
        description = "Paginated hadith family list (clusters of variants), sorted by \
                       variant_count DESC. Returns { data: [ApiHadithFamily...], page, limit, has_more }."
    )]
    async fn list_families(
        &self,
        Parameters(args): Parameters<PageLimitArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page = args.page.unwrap_or(1).max(1) as usize;
        let limit = clamp_limit(args.limit);
        let resp = services::family::list(&self.state, page, limit)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(
        description = "Fetch one hadith family by id with all its variants (hadiths) ordered by \
                       hadith_number."
    )]
    async fn get_family(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let detail = services::family::get_detail(&self.state, &args.id)
            .await
            .map_err(|e| {
                if services::family::is_not_found(&e) {
                    McpError::invalid_request(e.to_string(), None)
                } else {
                    mcp_err(e)
                }
            })?;
        Ok(CallToolResult::success(vec![Content::json(detail)?]))
    }

    #[tool(
        description = "Aggregate mustalah counts across all analyzed families: family_count, \
                       analyzed_count, and per-class (mutawatir/mashhur/aziz/gharib) totals."
    )]
    async fn get_mustalah_stats(&self) -> Result<CallToolResult, McpError> {
        let stats = services::family::mustalah_stats(&self.state)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(stats)?]))
    }

    #[tool(
        description = "Full structural mustalah analysis bundle for one family: { analysis: \
                       { breadth_class, min_breadth, bottleneck_tabaqah, chain_count, ilal_flags }, \
                       chains: [{ variant_id, narrator_count, has_chronology_conflict, narrator_ids }], \
                       pivots: [{ narrator_id, bundle_coverage, fan_out, collector_diversity, bypass_count, is_bottleneck }] } \
                       — the LLM-grade input for hadith authenticity reasoning."
    )]
    async fn analyze_family_mustalah(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let resp = services::family::mustalah_family_analysis(&self.state, &args.id)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    // ── Narrator (structured tool surface — name-resolves first, returns ToolOutput) ─

    #[tool(
        description = "Look up a narrator by name and return biographical info \
                       (name_ar, name_en, kunya, generation/tabaqah, death_year, bio, hadith_count)."
    )]
    async fn narrator_info(
        &self,
        Parameters(args): Parameters<NarratorByNameArgs>,
    ) -> Result<CallToolResult, McpError> {
        let n = resolve_or_404(&self.state.db, &args.name).await?;
        let out = services::narrator::narrator_info(&self.state.db, &n)
            .await
            .map_err(mcp_err)?;
        tool_output_json(out)
    }

    #[tool(
        description = "Find a narrator by name and return their teachers (heard_from edges). \
                       Returns formatted context plus structured narrator_sources."
    )]
    async fn narrator_teachers(
        &self,
        Parameters(args): Parameters<NarratorByNameArgs>,
    ) -> Result<CallToolResult, McpError> {
        let n = resolve_or_404(&self.state.db, &args.name).await?;
        let out = services::narrator::narrator_teachers(&self.state.db, &n)
            .await
            .map_err(mcp_err)?;
        tool_output_json(out)
    }

    #[tool(description = "Find a narrator by name and return their students.")]
    async fn narrator_students(
        &self,
        Parameters(args): Parameters<NarratorByNameArgs>,
    ) -> Result<CallToolResult, McpError> {
        let n = resolve_or_404(&self.state.db, &args.name).await?;
        let out = services::narrator::narrator_students(&self.state.db, &n)
            .await
            .map_err(mcp_err)?;
        tool_output_json(out)
    }

    #[tool(
        description = "Count hadiths narrated by a person, optionally filtered to a specific \
                       book (e.g. \"Bukhari\"). Uses the pre-computed `hadith_count` field when \
                       no book filter is supplied."
    )]
    async fn narrator_hadith_count(
        &self,
        Parameters(args): Parameters<NarratorCountArgs>,
    ) -> Result<CallToolResult, McpError> {
        let n = resolve_or_404(&self.state.db, &args.name).await?;
        let out = services::narrator::count_hadiths(&self.state.db, &n, args.book.as_deref())
            .await
            .map_err(mcp_err)?;
        tool_output_json(out)
    }

    // ── Classifier (introspection) ─────────────────────────────────────────

    #[tool(
        description = "Classify a free-form question into the in-house QueryIntent enum used by \
                       the agentic RAG dispatcher. Useful for transparency / debugging — shows \
                       what the system would route the question to."
    )]
    async fn classify_question(
        &self,
        Parameters(args): Parameters<ClassifyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let llm = require_llm(&self.state)?;
        let opts = crate::llm::ChatOptions::default();
        let intent = services::classify::classify(llm, &args.question, &opts)
            .await
            .map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::json(
            serde_json::json!({
                "intent": format!("{intent:?}"),
            }),
        )?]))
    }

    // ── Ask / RAG (collapse SSE stream into one JSON payload) ──────────────

    #[tool(
        description = "Ask a hadith-scoped question. Runs the agentic classify-then-execute \
                       pipeline (structured narrator queries when applicable, semantic RAG \
                       otherwise) and returns the full answer plus sources."
    )]
    async fn ask_hadith(
        &self,
        Parameters(args): Parameters<AskArgs>,
    ) -> Result<CallToolResult, McpError> {
        let llm = require_llm(&self.state)?;
        let embedder = require_embedder(&self.state)?;
        let opts = crate::llm::ChatOptions {
            model: args.model,
            ..Default::default()
        };
        let result = services::ask::ask_agentic(
            llm,
            &self.state.db,
            embedder,
            &args.question,
            &opts,
            services::ask::AskScope::Hadith,
        )
        .await
        .map_err(mcp_err)?;
        agentic_result_to_json(result).await
    }

    #[tool(
        description = "Ask a Quran-scoped question. Retrieves relevant ayahs + Ibn Kathir tafsir, \
                       streams a grounded answer, returns the full answer plus ayah sources."
    )]
    async fn ask_quran(
        &self,
        Parameters(args): Parameters<AskArgs>,
    ) -> Result<CallToolResult, McpError> {
        let llm = require_llm(&self.state)?;
        let embedder = require_embedder(&self.state)?;
        let opts = crate::llm::ChatOptions {
            model: args.model,
            ..Default::default()
        };
        let (ayah_sources, stream) =
            services::ask::ask_quran_only(llm, &self.state.db, embedder, &args.question, &opts)
                .await
                .map_err(mcp_err)?;
        let answer = drain_stream(stream).await?;
        Ok(CallToolResult::success(vec![Content::json(
            serde_json::json!({
                "answer": answer,
                "ayah_sources": ayah_sources,
            }),
        )?]))
    }

    #[tool(
        description = "Ask a question grounded in BOTH Quran and hadith corpora. Routes through \
                       the agentic dispatcher; falls back to combined semantic RAG when no \
                       structured intent matches."
    )]
    async fn ask_unified(
        &self,
        Parameters(args): Parameters<AskArgs>,
    ) -> Result<CallToolResult, McpError> {
        let llm = require_llm(&self.state)?;
        let embedder = require_embedder(&self.state)?;
        let opts = crate::llm::ChatOptions {
            model: args.model,
            ..Default::default()
        };
        let result = services::ask::ask_agentic(
            llm,
            &self.state.db,
            embedder,
            &args.question,
            &opts,
            services::ask::AskScope::Both,
        )
        .await
        .map_err(mcp_err)?;
        agentic_result_to_json(result).await
    }

    // ── Notes (read tools — always enabled) ────────────────────────────────

    #[tool(
        description = "Paginated list of user notes. Optional filters: ref_type, ref_id, color, q \
                       (free-text on title+content), tag, notebook_id (or \"__uncategorized__\")."
    )]
    async fn list_notes(
        &self,
        Parameters(args): Parameters<ListNotesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page = args.page.unwrap_or(1).max(1) as usize;
        let limit = args.limit.unwrap_or(20).clamp(1, 200) as usize;
        let filter = services::notes::NoteListFilter {
            ref_type: args.ref_type,
            ref_id: args.ref_id,
            color: args.color,
            q: args.q,
            tag: args.tag,
            notebook_id: args.notebook_id,
        };
        let resp = services::notes::list(&self.state, &filter, page, limit)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(resp)?]))
    }

    #[tool(description = "Fetch a single note by id.")]
    async fn get_note(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let n = services::notes::get(&self.state, &args.id)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(n)?]))
    }

    #[tool(
        description = "For a list of (ref_type, ref_id) pairs, return per-ref note count + color. \
                       Map key is ref_id; value is { color, count }."
    )]
    async fn bulk_note_refs(
        &self,
        Parameters(args): Parameters<BulkRefsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let v = services::notes::bulk_refs(&self.state, &args.ref_type, &args.ref_ids)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(v)?]))
    }

    #[tool(description = "Distinct sorted list of all tags currently used across notes.")]
    async fn list_note_tags(&self) -> Result<CallToolResult, McpError> {
        let tags = services::notes::list_tags(&self.state)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(tags)?]))
    }

    #[tool(description = "Export every note, ordered by updated_at DESC.")]
    async fn export_notes(&self) -> Result<CallToolResult, McpError> {
        let notes = services::notes::export(&self.state)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(notes)?]))
    }

    #[tool(description = "List all notebooks, ordered by sort_order then created_at.")]
    async fn list_notebooks(&self) -> Result<CallToolResult, McpError> {
        let nbs = services::notes::list_notebooks(&self.state)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(nbs)?]))
    }

    // ── Notes (mutation tools) ─────────────────────────────────────────────

    #[tool(description = "Create a new note.")]
    async fn create_note(
        &self,
        Parameters(args): Parameters<CreateNoteArgs>,
    ) -> Result<CallToolResult, McpError> {
        let n = services::notes::create(
            &self.state,
            args.ref_type,
            args.ref_id,
            args.title,
            args.content,
            args.color,
            args.tags,
            args.refs,
            args.notebook_id,
        )
        .await
        .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(n)?]))
    }

    #[tool(description = "Update fields on a note. At least one field required.")]
    async fn update_note(
        &self,
        Parameters(args): Parameters<UpdateNoteArgs>,
    ) -> Result<CallToolResult, McpError> {
        let patch = services::notes::NotePatch {
            title: args.title,
            content: args.content,
            color: args.color,
            tags: args.tags,
            refs: args.refs,
            notebook_id: args.notebook_id,
        };
        let n = services::notes::update(&self.state, &args.id, patch)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(n)?]))
    }

    #[tool(description = "Delete a note by id.")]
    async fn delete_note(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        services::notes::delete(&self.state, &args.id)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(
            serde_json::json!({"ok": true}),
        )?]))
    }

    #[tool(
        description = "Add or remove a single ref from a note. action must be \"add\" or \"remove\". \
                      "
    )]
    async fn update_note_refs(
        &self,
        Parameters(args): Parameters<UpdateNoteRefsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let n =
            services::notes::update_note_refs(&self.state, &args.id, &args.action, args.note_ref)
                .await
                .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(n)?]))
    }

    #[tool(description = "Replace the annotation on an existing ref by index.")]
    async fn update_ref_annotation(
        &self,
        Parameters(args): Parameters<UpdateRefAnnotationArgs>,
    ) -> Result<CallToolResult, McpError> {
        let n = services::notes::update_ref_annotation(
            &self.state,
            &args.id,
            args.idx as usize,
            args.annotation,
        )
        .await
        .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(n)?]))
    }

    #[tool(description = "Create a new notebook.")]
    async fn create_notebook(
        &self,
        Parameters(args): Parameters<CreateNotebookArgs>,
    ) -> Result<CallToolResult, McpError> {
        let nb =
            services::notes::create_notebook(&self.state, args.name, args.emoji, args.parent_id)
                .await
                .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(nb)?]))
    }

    #[tool(description = "Update a notebook. At least one field required.")]
    async fn update_notebook(
        &self,
        Parameters(args): Parameters<UpdateNotebookArgs>,
    ) -> Result<CallToolResult, McpError> {
        let patch = services::notes::NotebookPatch {
            name: args.name,
            emoji: args.emoji,
            parent_id: args.parent_id,
            sort_order: args.sort_order,
        };
        let nb = services::notes::update_notebook(&self.state, &args.id, patch)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(nb)?]))
    }

    #[tool(description = "Delete a notebook + clear its references from notes/child notebooks.")]
    async fn delete_notebook(
        &self,
        Parameters(args): Parameters<GetByIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        services::notes::delete_notebook(&self.state, &args.id)
            .await
            .map_err(note_err)?;
        Ok(CallToolResult::success(vec![Content::json(
            serde_json::json!({"ok": true}),
        )?]))
    }
}

// ── Output helpers ──────────────────────────────────────────────────────────

fn tool_output_json(out: services::narrator::ToolOutput) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::json(
        serde_json::json!({
            "context": out.context,
            "narrator_sources": out.narrator_sources,
            "hadith_sources": out.hadith_sources,
        }),
    )?]))
}

async fn resolve_or_404(
    db: &surrealdb::Surreal<crate::db::Db>,
    name: &str,
) -> Result<crate::models::Narrator, McpError> {
    services::narrator::resolve_narrator(db, name)
        .await
        .map_err(mcp_err)?
        .ok_or_else(|| McpError::invalid_request(format!("narrator '{name}' not found"), None))
}

async fn agentic_result_to_json(
    result: services::ask::AgenticResult,
) -> Result<CallToolResult, McpError> {
    let payload = match result {
        services::ask::AgenticResult::Structured {
            narrator_sources,
            hadith_sources,
            token_stream,
        } => {
            let answer = drain_stream(token_stream).await?;
            serde_json::json!({
                "answer": answer,
                "narrator_sources": narrator_sources,
                "hadith_sources": hadith_sources,
            })
        }
        services::ask::AgenticResult::Semantic {
            ayah_sources,
            hadith_sources,
            token_stream,
        } => {
            let answer = drain_stream(token_stream).await?;
            serde_json::json!({
                "answer": answer,
                "ayah_sources": ayah_sources,
                "hadith_sources": hadith_sources,
            })
        }
    };
    Ok(CallToolResult::success(vec![Content::json(payload)?]))
}
