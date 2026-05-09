use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use serde::Deserialize;
use surrealdb::types::RecordId;
use utoipa::{IntoParams, ToSchema};

use super::sse::token_stream_to_sse;
use crate::analysis;
use crate::llm::ChatOptions;
use crate::models::{
    ApiCollection, ApiHadithSearchResult, ApiNarratorWithCount, CommonNarratorsResponse, GraphData,
    IsnadSearchResponse, StatsResponse,
};

use super::AppState;

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

/// Server capabilities — which optional features are wired up at runtime.
///
/// Useful for clients to know which providers (LLM, embedder, reranker) are
/// available before calling search/ask endpoints.
#[utoipa::path(
    get,
    path = "/config",
    tag = "Meta",
    responses((status = 200, description = "Capability flags", body = serde_json::Value))
)]
pub async fn app_config(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::to_value(crate::services::meta::app_config(&state)).unwrap())
}

// ── Query parameter types ──

#[derive(Deserialize, IntoParams)]
pub struct SearchParams {
    /// Free-text query. Empty/missing returns an empty result set.
    pub q: Option<String>,
    /// Search mode: `text` (BM25), `semantic` (vector), or `hybrid` (RRF). Defaults to `hybrid`.
    #[serde(rename = "type")]
    #[param(rename = "type")]
    pub search_type: Option<String>,
    pub limit: Option<usize>,
    pub page: Option<usize>,
    /// Opt-in cross-encoder reranking. Only honoured for `type=hybrid` AND
    /// when the server was started with `--reranker`. Otherwise ignored.
    pub rerank: Option<bool>,
}

#[derive(Deserialize, IntoParams)]
pub struct ListParams {
    /// Numeric `collection_id` (1=Bukhari, 2=Muslim, 3=Abu Dawud, 4=Tirmidhi, 5=Nasai, 6=Ibn Majah).
    pub book: Option<i64>,
    /// Filter by `hadith_number` within a collection.
    pub number: Option<i64>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
    /// Free-text substring filter on Arabic / English text.
    pub q: Option<String>,
    /// Filter by narrator generation (tabaqah).
    pub generation: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct AskRequest {
    pub question: String,
    /// Optional model override (e.g. `llama3.2`, `gpt-4o-mini`). Defaults to the server's configured model.
    pub model: Option<String>,
}

#[derive(Deserialize, IntoParams)]
pub struct AutocompleteParams {
    pub q: String,
    pub limit: Option<usize>,
}

#[derive(Deserialize, ToSchema)]
pub struct IsnadSearchRequest {
    /// Narrator IDs that must appear in the chain.
    pub narrator_ids: Vec<String>,
    /// `loose` (default) — narrators may appear in any order; `strict` — narrators must form a contiguous sub-chain.
    pub mode: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize, IntoParams)]
pub struct CommonNarratorsParams {
    /// First narrator ID.
    pub a: String,
    /// Second narrator ID.
    pub b: String,
}

// ── API Handlers ──

/// Top-line counts (hadiths, narrators, collections).
#[utoipa::path(
    get,
    path = "/stats",
    tag = "Meta",
    responses((status = 200, body = StatsResponse))
)]
pub async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    match crate::services::meta::stats(&state).await {
        Ok(s) => Json(s),
        Err(e) => {
            tracing::error!("Stats query failed: {e}");
            Json(StatsResponse {
                hadith_count: 0,
                narrator_count: 0,
                book_count: 0,
            })
        }
    }
}

/// List the canonical hadith collections (Kutub al-Sittah). Stable IDs come
/// from `src/ingest/books.rs` — `bukhari` (1), `muslim` (2), `abudawud` (3),
/// `tirmidhi` (4), `nasai` (5), `ibnmajah` (6).
#[utoipa::path(
    get,
    path = "/collections",
    tag = "Hadith",
    responses((status = 200, body = Vec<ApiCollection>))
)]
pub async fn books(State(state): State<AppState>) -> impl IntoResponse {
    match crate::services::hadith::list_collections(&state).await {
        Ok(list) => Json(list),
        Err(e) => {
            tracing::error!("Books query failed: {e}");
            Json(Vec::<ApiCollection>::new())
        }
    }
}

/// Search hadiths and narrators in a single call.
///
/// `type=text` uses BM25 full-text; `type=semantic` uses HNSW vector search;
/// `type=hybrid` (default) fuses both via Reciprocal Rank Fusion. Set
/// `?rerank=true` to apply a cross-encoder reranker on hybrid results when the
/// server is configured with `--reranker`.
#[utoipa::path(
    get,
    path = "/search/hadith",
    tag = "Search",
    params(SearchParams),
    responses((status = 200, description = "Hadith + narrator search results", body = serde_json::Value))
)]
pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    let mode = crate::services::search::SearchMode::parse(params.search_type.as_deref())
        .unwrap_or(crate::services::search::SearchMode::Hybrid);
    let limit = params.limit.unwrap_or(20);
    let rerank = params.rerank.unwrap_or(false);
    match crate::services::search::search_hadith_and_narrators(&state, &query, mode, limit, rerank)
        .await
    {
        Ok(resp) => Json(serde_json::to_value(resp).unwrap()),
        Err(e) => {
            tracing::error!("Combined search failed: {e}");
            Json(serde_json::json!({
                "query": query,
                "search_type": format!("{mode:?}").to_lowercase(),
                "hadiths": [],
                "narrators": [],
            }))
        }
    }
}

/// Paginated list of hadiths with optional filters.
#[utoipa::path(
    get,
    path = "/hadiths",
    tag = "Hadith",
    params(ListParams),
    responses((status = 200, description = "{ data: ApiHadith[], page, limit, has_more, total? }", body = serde_json::Value))
)]
pub async fn hadith_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    match crate::services::hadith::list(&state, params.book, params.number, page, limit).await {
        Ok(resp) => Json(serde_json::to_value(resp).unwrap()),
        Err(e) => {
            tracing::error!("Hadith list query failed: {e}");
            Json(serde_json::json!({
                "data": [],
                "page": page,
                "limit": limit,
                "has_more": false,
            }))
        }
    }
}

/// Single hadith with its narrators, linked Quran ayahs, and similar hadiths.
#[utoipa::path(
    get,
    path = "/hadiths/{id}",
    tag = "Hadith",
    params(("id" = String, Path, description = "Hadith slug, e.g. `bukhari:1`")),
    responses((status = 200, description = "Hadith + narrators + linked ayahs + similar hadiths", body = serde_json::Value))
)]
pub async fn hadith_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    match crate::services::hadith::get_detail(&state, &id).await {
        Ok(resp) => Ok(Json(serde_json::to_value(resp).unwrap())),
        Err(e) if crate::services::hadith::is_not_found(&e) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Hadith detail query failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Paginated list of narrators. Supports free-text and generation filters.
#[utoipa::path(
    get,
    path = "/narrators",
    tag = "Narrators",
    params(
        ("q" = Option<String>, Query, description = "Free-text name filter"),
        ("generation" = Option<String>, Query, description = "Tabaqah filter (e.g. `1` for Sahabah)"),
        ("page" = Option<usize>, Query, description = "1-indexed page (default 1)"),
        ("limit" = Option<usize>, Query, description = "Page size (default 50)"),
    ),
    responses((status = 200, description = "{ data: ApiNarratorWithCount[], page, limit, has_more, total? }", body = serde_json::Value))
)]
pub async fn narrator_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(50);
    match crate::services::narrator::list(
        &state,
        params.q.as_deref(),
        params.generation.as_deref(),
        page,
        limit,
    )
    .await
    {
        Ok(resp) => Json(serde_json::to_value(resp).unwrap()),
        Err(e) => {
            tracing::error!("Narrator list query failed: {e}");
            Json(serde_json::json!({
                "data": [],
                "page": page,
                "limit": limit,
                "has_more": false,
            }))
        }
    }
}

/// Single narrator with biographical fields, sample hadiths, teachers, and students.
#[utoipa::path(
    get,
    path = "/narrators/{id}",
    tag = "Narrators",
    params(("id" = String, Path, description = "Narrator ID")),
    responses((status = 200, description = "Narrator + hadiths + teachers + students", body = serde_json::Value))
)]
pub async fn narrator_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    match crate::services::narrator::get_detail(&state, &id).await {
        Ok(resp) => Ok(Json(serde_json::to_value(resp).unwrap())),
        Err(e) if crate::services::narrator::is_not_found(&e) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Narrator detail query failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Cytoscape-shaped isnad graph for a hadith — nodes for each narrator in the
/// chain plus edges marked with `chain_position`.
#[utoipa::path(
    get,
    path = "/hadiths/{id}/chain",
    tag = "Hadith",
    params(("id" = String, Path, description = "Hadith slug, e.g. `bukhari:1`")),
    responses((status = 200, body = GraphData))
)]
pub async fn chain_graph_data(
    State(state): State<AppState>,
    Path(hadith_id): Path<String>,
) -> impl IntoResponse {
    match crate::services::hadith::get_chain_graph(&state, &hadith_id).await {
        Ok(g) => Json(g),
        Err(e) => {
            tracing::error!("Chain graph query failed: {e}");
            Json(GraphData {
                nodes: vec![],
                edges: vec![],
                total_teachers: None,
                total_students: None,
            })
        }
    }
}

/// Cytoscape-shaped narrator network — the centre narrator plus their
/// immediate teachers (incoming) and students (outgoing). Capped at 25 nodes.
#[utoipa::path(
    get,
    path = "/narrators/{id}/graph",
    tag = "Narrators",
    params(("id" = String, Path)),
    responses((status = 200, body = GraphData))
)]
pub async fn narrator_graph_data(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match crate::services::narrator::get_graph(&state, &id).await {
        Ok(g) => Json(g),
        Err(e) => {
            tracing::error!("Narrator graph query failed: {e}");
            Json(GraphData {
                nodes: vec![],
                edges: vec![],
                total_teachers: None,
                total_students: None,
            })
        }
    }
}

/// Hadith-grounded GraphRAG question answering.
///
/// **Streaming response** (SSE, `text/event-stream`): the first event carries
/// `{narrator_sources, hadith_sources}`; subsequent events stream tokens from
/// the LLM. Rate-limited to ~10 req/min per IP because each call hits the LLM.
#[utoipa::path(
    post,
    path = "/ask/hadith",
    tag = "Ask",
    request_body = AskRequest,
    responses((status = 200, description = "SSE token stream with sources prefix", body = serde_json::Value, content_type = "text/event-stream"))
)]
pub async fn ask(
    State(state): State<AppState>,
    Json(body): Json<AskRequest>,
) -> Result<Response, StatusCode> {
    let question = body.question.trim().to_string();
    if question.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let embedder = state.embedder.as_ref().ok_or_else(|| {
        tracing::error!("Embeddings provider not available");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    let llm = state.llm.as_ref().ok_or_else(|| {
        tracing::error!("LLM provider not configured");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    let opts = ChatOptions {
        model: body.model.clone(),
        ..Default::default()
    };

    let result = crate::services::ask::ask_agentic(
        llm.as_ref(),
        &state.db,
        embedder.as_ref(),
        &question,
        &opts,
        crate::services::ask::AskScope::Hadith,
    )
    .await
    .map_err(|e| {
        tracing::error!("Agentic RAG ask (hadith) failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    use crate::services::ask::AgenticResult;

    let (sources_event, token_stream) = match result {
        AgenticResult::Structured {
            narrator_sources,
            hadith_sources,
            token_stream,
        } => {
            let hadith_api: Vec<ApiHadithSearchResult> = hadith_sources
                .into_iter()
                .map(ApiHadithSearchResult::from)
                .collect();
            let event = format!(
                "data: {}\n\n",
                serde_json::to_string(&serde_json::json!({
                    "narrator_sources": narrator_sources,
                    "hadith_sources": hadith_api,
                }))
                .unwrap()
            );
            (event, token_stream)
        }
        AgenticResult::Semantic {
            hadith_sources,
            token_stream,
            ..
        } => {
            let hadith_api: Vec<ApiHadithSearchResult> = hadith_sources
                .into_iter()
                .map(ApiHadithSearchResult::from)
                .collect();
            let event = format!(
                "data: {}\n\n",
                serde_json::to_string(&serde_json::json!({ "hadith_sources": hadith_api }))
                    .unwrap()
            );
            (event, token_stream)
        }
    };

    let sse_stream =
        futures::stream::once(
            async move { Ok::<_, std::io::Error>(bytes::Bytes::from(sources_event)) },
        )
        .chain(token_stream_to_sse(token_stream));

    let body = Body::from_stream(sse_stream);

    Ok(Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap())
}

// ── Internal translation update endpoint ──

// ── Narrator update endpoint ──

#[derive(Deserialize)]
pub struct UpdateNarratorRequest {
    pub name_ar: Option<String>,
    pub name_en: Option<String>,
    pub gender: Option<String>,
    pub generation: Option<String>,
    pub bio: Option<String>,
    pub kunya: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub birth_year: Option<i64>,
    pub birth_calendar: Option<String>,
    pub death_year: Option<i64>,
    pub death_calendar: Option<String>,
    pub locations: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

pub async fn update_narrator(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNarratorRequest>,
) -> impl IntoResponse {
    // Build a JSON object of all provided fields, then MERGE into the narrator
    let mut update = serde_json::Map::new();

    macro_rules! set_field {
        ($name:ident) => {
            if let Some(ref v) = body.$name {
                update.insert(stringify!($name).to_string(), serde_json::json!(v));
            }
        };
    }

    set_field!(name_ar);
    set_field!(name_en);
    set_field!(gender);
    set_field!(generation);
    set_field!(bio);
    set_field!(kunya);
    set_field!(aliases);
    set_field!(birth_year);
    set_field!(birth_calendar);
    set_field!(death_year);
    set_field!(death_calendar);
    set_field!(locations);
    set_field!(tags);
    if update.is_empty() {
        return StatusCode::BAD_REQUEST;
    }

    match state
        .db
        .query("UPDATE $rid MERGE $data")
        .bind(("rid", rid("narrator", &id)))
        .bind(("data", serde_json::Value::Object(update)))
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("Narrator update failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

// ── Internal translation update endpoint ──

#[derive(Deserialize)]
pub struct TranslateUpdate {
    pub table: String,
    pub id: String,
    pub field: String,
    pub value: String,
}

pub async fn update_translation(
    State(state): State<AppState>,
    Json(body): Json<TranslateUpdate>,
) -> impl IntoResponse {
    let table = &body.table;
    let field = &body.field;
    // Only allow updating specific fields on specific tables
    if !matches!(table.as_str(), "hadith" | "narrator")
        || !matches!(field.as_str(), "text_en" | "name_en")
    {
        return StatusCode::BAD_REQUEST;
    }

    let sql = format!("UPDATE $rid SET {field} = $value");
    match state
        .db
        .query(&sql)
        .bind(("rid", rid(table, &body.id)))
        .bind(("value", body.value.clone()))
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("Translation update failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

// ── Analysis endpoints ──

/// Paginated list of hadith families (variant clusters), sorted by variant count.
#[utoipa::path(
    get,
    path = "/families",
    tag = "Families",
    params(
        ("page" = Option<usize>, Query, description = "1-indexed page (default 1)"),
        ("limit" = Option<usize>, Query, description = "Page size (default 20)"),
    ),
    responses((status = 200, description = "{ data: ApiHadithFamily[], page, limit, has_more, total? }", body = serde_json::Value))
)]
pub async fn family_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    match crate::services::family::list(&state, page, limit).await {
        Ok(resp) => Json(serde_json::to_value(resp).unwrap()),
        Err(e) => {
            tracing::error!("Family list query failed: {e}");
            Json(serde_json::json!({
                "data": [],
                "page": page,
                "limit": limit,
                "has_more": false,
            }))
        }
    }
}

/// One hadith family with all variants.
#[utoipa::path(
    get,
    path = "/families/{id}",
    tag = "Families",
    params(("id" = String, Path)),
    responses((status = 200, description = "Family + variants", body = serde_json::Value))
)]
pub async fn family_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    match crate::services::family::get_detail(&state, &id).await {
        Ok(resp) => Ok(Json(serde_json::to_value(resp).unwrap())),
        Err(e) if crate::services::family::is_not_found(&e) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Family detail query failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ── Hadith gradings (multi-scholar verdicts) ──
//
// Returns one row per scholar per source book. For Bukhari/Muslim a synthetic
// "consensus sahih" row is prepended. The user explores narrator-level
// reliability by clicking through to each narrator's Tahdhib bio page — there
// is intentionally no automatic suspect-narrator surfacing here.

/// Multi-scholar verdicts on this hadith.
///
/// One row per scholar per source book. For Bukhari and Muslim hadiths the
/// response **prepends a synthetic `consensus sahih` row** with
/// `source_book_id=null`. Stored rows include the source `book_id` and
/// `page_index` so consumers can fetch the original Arabic via
/// `GET /v1/books/{source_book_id}/pages?page={source_page_index+1}`.
///
/// Multiple rows per `(hadith, scholar, source_book)` are intentional —
/// scholars like Albani regularly issue distinct verdicts on different chains
/// within a single book entry.
#[utoipa::path(
    get,
    path = "/hadiths/{id}/gradings",
    tag = "Hadith",
    params(("id" = String, Path, description = "Hadith slug, e.g. `bukhari:1`")),
    responses((status = 200, body = crate::models::ApiHadithGradingsResponse))
)]
pub async fn hadith_gradings(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match crate::services::scholars::get_hadith_gradings(&state, &id).await {
        Ok(resp) => Json(resp),
        Err(e) => {
            tracing::warn!("hadith_gradings query failed: {e}");
            Json(crate::models::ApiHadithGradingsResponse {
                hadith_id: id,
                gradings: vec![],
            })
        }
    }
}

// ── Distinct list of scholars who have at least one stored verdict ──

/// Distinct scholars who have at least one stored verdict, with verdict count.
///
/// Useful as a filter UI source for clients building scholar-leaderboard or
/// per-scholar exploration pages.
#[utoipa::path(
    get,
    path = "/scholars",
    tag = "Hadith",
    responses((status = 200, body = Vec<crate::models::ApiScholar>))
)]
pub async fn list_scholars(State(state): State<AppState>) -> impl IntoResponse {
    match crate::services::scholars::list_scholars(&state).await {
        Ok(s) => Json(s),
        Err(e) => {
            tracing::warn!("list_scholars: query failed: {e}");
            Json(Vec::<crate::models::ApiScholar>::new())
        }
    }
}

// ── Mustalah API handlers ──

/// Aggregate counts across mustalah breadth classifications
/// (mutawatir / mashhur / aziz / gharib) for all analyzed families.
#[utoipa::path(
    get,
    path = "/mustalah/stats",
    tag = "Mustalah",
    responses((status = 200, description = "Per-class family counts", body = serde_json::Value))
)]
pub async fn mustalah_stats(State(state): State<AppState>) -> impl IntoResponse {
    match crate::services::family::mustalah_stats(&state).await {
        Ok(stats) => Json(serde_json::to_value(stats).unwrap()),
        Err(e) => {
            tracing::error!("Mustalah stats query failed: {e}");
            Json(serde_json::json!({
                "family_count": 0, "analyzed_count": 0,
                "mutawatir_count": 0, "mashhur_count": 0,
                "aziz_count": 0, "gharib_count": 0,
            }))
        }
    }
}

/// Full mustalah analysis for one hadith family.
///
/// Bundles `isnad_analysis` (breadth, bottleneck tabaqah, sahabi/mutabaat/shawahid
/// counts, ilal flags), per-chain `chain_assessment` rows, and the
/// `narrator_pivot` rows that mark *madar al-isnad* — the key narrators every
/// chain runs through.
#[utoipa::path(
    get,
    path = "/families/{id}/mustalah",
    tag = "Mustalah",
    params(("id" = String, Path)),
    responses((status = 200, description = "Family-level isnad analysis bundle", body = serde_json::Value))
)]
pub async fn mustalah_family_analysis(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    match crate::services::family::mustalah_family_analysis(&state, &id).await {
        Ok(resp) => Ok(Json(serde_json::to_value(resp).unwrap())),
        Err(e) => {
            tracing::error!("Mustalah family analysis query failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// How often this narrator acts as a pivot or bottleneck across families.
#[utoipa::path(
    get,
    path = "/narrators/{id}/isnad-role",
    tag = "Narrators",
    params(("id" = String, Path)),
    responses((status = 200, description = "Pivot / bottleneck counts + family list", body = serde_json::Value))
)]
pub async fn narrator_isnad_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match crate::services::narrator::get_isnad_role(&state, &id).await {
        Ok(role) => Json(serde_json::to_value(role).unwrap()),
        Err(e) => {
            tracing::error!("Narrator isnad role query failed: {e}");
            Json(serde_json::json!({
                "narrator_id": id,
                "pivot_family_count": 0,
                "bottleneck_family_count": 0,
                "families": [],
            }))
        }
    }
}

/// Word-level matn diff between two hadiths. Useful for studying narrator
/// paraphrases across variants of the same family.
#[utoipa::path(
    get,
    path = "/hadiths/diff",
    tag = "Hadith",
    params(DiffParams),
    responses((status = 200, description = "Diff result with highlighted text spans", body = serde_json::Value))
)]
pub async fn matn_diff_handler(
    State(state): State<AppState>,
    Query(params): Query<DiffParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let a_id = params.a.ok_or(StatusCode::BAD_REQUEST)?;
    let b_id = params.b.ok_or(StatusCode::BAD_REQUEST)?;
    match crate::services::hadith::matn_diff(&state, &a_id, &b_id).await {
        Ok(result) => Ok(Json(result)),
        Err(e) if crate::services::hadith::is_not_found(&e) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Matn diff failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct DiffParams {
    /// First hadith ID (e.g. `bukhari:1`).
    pub a: Option<String>,
    /// Second hadith ID.
    pub b: Option<String>,
}

/// Export a complete family analysis bundle.
///
/// `?format=json` (default) returns a structured `ArtifactBundle`;
/// `?format=md` returns a single human-readable Markdown document with
/// `Content-Disposition: attachment`.
#[utoipa::path(
    get,
    path = "/families/{id}/export",
    tag = "Families",
    params(
        ("id" = String, Path),
        ExportParams,
    ),
    responses(
        (status = 200, description = "ArtifactBundle JSON or Markdown document", body = serde_json::Value),
        (status = 404, description = "Family not found")
    )
)]
pub async fn export_family(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let result = analysis::export::fetch_family_analysis(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let format = params.format.as_deref().unwrap_or("json");

    match format {
        "md" | "markdown" => {
            let md = analysis::export::export_markdown(&result);
            Ok(Response::builder()
                .header("Content-Type", "text/markdown")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"family_{id}.md\""),
                )
                .body(Body::from(md))
                .unwrap())
        }
        _ => {
            let bundle = analysis::export::ArtifactBundle::from(&result);
            let json = serde_json::to_string_pretty(&bundle).unwrap_or_default();
            Ok(Response::builder()
                .header("Content-Type", "application/json")
                .body(Body::from(json))
                .unwrap())
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct ExportParams {
    /// `json` (default) — structured ArtifactBundle; `md` — single Markdown document.
    pub format: Option<String>,
}

// Helper result types (CountResult, NarratorsResult, HadithsResult, etc.)
// have moved to their owning service modules (services::hadith, ::narrator,
// ::family, ::scholars, ::meta) — handlers no longer marshal raw rows.

// ── Isnad Search endpoints ──

/// Typeahead autocomplete over narrator names (English, Arabic, kunya, aliases).
#[utoipa::path(
    get,
    path = "/narrators/autocomplete",
    tag = "Narrators",
    params(AutocompleteParams),
    responses((status = 200, body = Vec<ApiNarratorWithCount>))
)]
pub async fn narrator_autocomplete(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(8);
    match crate::services::narrator::autocomplete(&state, &params.q, limit).await {
        Ok(rows) => Json(rows),
        Err(e) => {
            tracing::error!("Autocomplete query failed: {e}");
            Json(Vec::<ApiNarratorWithCount>::new())
        }
    }
}

/// Find hadiths whose isnad chain contains the given narrators.
///
/// `mode: "loose"` (default) — narrators may appear in any order anywhere in
/// the chain. `mode: "strict"` — narrators must form a contiguous sub-chain in
/// the order provided.
#[utoipa::path(
    post,
    path = "/isnad/search",
    tag = "Isnad",
    request_body = IsnadSearchRequest,
    responses((status = 200, body = IsnadSearchResponse))
)]
pub async fn isnad_search(
    State(state): State<AppState>,
    Json(body): Json<IsnadSearchRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let limit = body.limit.unwrap_or(20);
    let mode = body.mode.as_deref().unwrap_or("loose");
    match crate::services::hadith::isnad_search(&state, &body.narrator_ids, mode, limit).await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) if crate::services::hadith::is_not_found(&e) => Err(StatusCode::NOT_FOUND),
        Err(e) if crate::services::hadith::is_bad_request(&e) => Err(StatusCode::BAD_REQUEST),
        Err(e) => {
            tracing::error!("Isnad search failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Narrators who appear in chains of hadiths narrated by both `a` and `b`.
#[utoipa::path(
    get,
    path = "/narrators/common",
    tag = "Narrators",
    params(CommonNarratorsParams),
    responses((status = 200, body = CommonNarratorsResponse))
)]
pub async fn common_narrators(
    State(state): State<AppState>,
    Query(params): Query<CommonNarratorsParams>,
) -> Result<impl IntoResponse, StatusCode> {
    match crate::services::narrator::list_common(&state, &params.a, &params.b).await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) if crate::services::narrator::is_not_found(&e) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Common narrators failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ── Unified Quran & Sunnah endpoints ──

/// Cross-domain search across both Quran ayahs and Hadiths in one call.
#[utoipa::path(
    get,
    path = "/search/all",
    tag = "Search",
    params(SearchParams),
    responses((status = 200, description = "Mixed Quran + Hadith results with separate counts", body = serde_json::Value))
)]
pub async fn unified_search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    let search_type = params.search_type.unwrap_or_else(|| "hybrid".into());
    let limit = params.limit.unwrap_or(20);
    let page = params.page.unwrap_or(1).max(1);
    let rerank = params.rerank.unwrap_or(false);

    if query.is_empty() {
        return Json(serde_json::json!({
            "query": query,
            "search_type": search_type,
            "results": [],
            "quran_count": 0,
            "hadith_count": 0,
            "page": page,
            "has_more": false
        }));
    }

    let reranker = if rerank && search_type == "hybrid" {
        state.reranker.as_deref()
    } else {
        None
    };

    let effective_type =
        if state.embedder.is_none() && (search_type == "semantic" || search_type == "hybrid") {
            "text".to_string()
        } else {
            search_type.clone()
        };

    let Some(ref embedder) = state.embedder else {
        // Advanced disabled — text-only unified search inlined
        match crate::services::search::search_unified_text_only(&state.db, &query, limit, page)
            .await
        {
            Ok(response) => return Json(serde_json::to_value(response).unwrap()),
            Err(e) => {
                tracing::error!("Unified text search failed: {e}");
                return Json(serde_json::json!({
                    "query": query, "search_type": "text", "results": [],
                    "quran_count": 0, "hadith_count": 0
                }));
            }
        }
    };

    match crate::services::search::search_unified(
        &state.db,
        embedder.as_ref(),
        &query,
        &effective_type,
        limit,
        page,
        reranker,
    )
    .await
    {
        Ok(response) => Json(serde_json::to_value(response).unwrap()),
        Err(e) => {
            tracing::error!("Unified search failed: {e}");
            Json(serde_json::json!({
                "query": query,
                "search_type": "hybrid",
                "results": [],
                "quran_count": 0,
                "hadith_count": 0
            }))
        }
    }
}

/// Cross-domain GraphRAG question answering over Quran + Hadith.
///
/// **Streaming response** (SSE). Sources event includes Quran ayahs OR
/// hadiths (and narrator chains) depending on the classifier output.
/// Rate-limited harder than read endpoints.
#[utoipa::path(
    post,
    path = "/ask/all",
    tag = "Ask",
    request_body = AskRequest,
    responses((status = 200, description = "SSE token stream with sources prefix", body = serde_json::Value, content_type = "text/event-stream"))
)]
pub async fn unified_ask(
    State(state): State<AppState>,
    Json(body): Json<AskRequest>,
) -> Result<Response, StatusCode> {
    let question = body.question.trim().to_string();
    if question.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let embedder = state.embedder.as_ref().ok_or_else(|| {
        tracing::error!("Embeddings provider not available");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    let llm = state.llm.as_ref().ok_or_else(|| {
        tracing::error!("LLM provider not configured");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    let opts = ChatOptions {
        model: body.model.clone(),
        ..Default::default()
    };

    let result = crate::services::ask::ask_agentic(
        llm.as_ref(),
        &state.db,
        embedder.as_ref(),
        &question,
        &opts,
        crate::services::ask::AskScope::Both,
    )
    .await
    .map_err(|e| {
        tracing::error!("Agentic RAG ask failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    use crate::quran::models::ApiAyahSearchResult;
    use crate::services::ask::AgenticResult;

    let (sources_event, token_stream) = match result {
        AgenticResult::Structured {
            narrator_sources,
            hadith_sources,
            token_stream,
        } => {
            let hadith_api: Vec<ApiHadithSearchResult> = hadith_sources
                .into_iter()
                .map(ApiHadithSearchResult::from)
                .collect();
            let event = format!(
                "data: {}\n\n",
                serde_json::to_string(&serde_json::json!({
                    "narrator_sources": narrator_sources,
                    "hadith_sources": hadith_api,
                }))
                .unwrap()
            );
            (event, token_stream)
        }
        AgenticResult::Semantic {
            ayah_sources,
            hadith_sources,
            token_stream,
        } => {
            let quran_api: Vec<ApiAyahSearchResult> = ayah_sources
                .into_iter()
                .map(ApiAyahSearchResult::from)
                .collect();
            let hadith_api: Vec<ApiHadithSearchResult> = hadith_sources
                .into_iter()
                .map(ApiHadithSearchResult::from)
                .collect();
            let event = format!(
                "data: {}\n\n",
                serde_json::to_string(&serde_json::json!({
                    "quran_sources": quran_api,
                    "hadith_sources": hadith_api,
                }))
                .unwrap()
            );
            (event, token_stream)
        }
    };

    let sse_stream =
        futures::stream::once(
            async move { Ok::<_, std::io::Error>(bytes::Bytes::from(sources_event)) },
        )
        .chain(token_stream_to_sse(token_stream));

    let body = Body::from_stream(sse_stream);

    Ok(Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap())
}

// ── Link Preview ──

#[derive(Deserialize)]
pub struct LinkPreviewParams {
    pub url: String,
}

pub async fn link_preview(
    State(state): State<AppState>,
    Query(params): Query<LinkPreviewParams>,
) -> Result<impl IntoResponse, StatusCode> {
    use crate::models::{ApiLinkPreview, LinkPreview};

    let url = params.url.trim().to_string();
    if url.is_empty() || (!url.starts_with("http://") && !url.starts_with("https://")) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check cache first
    let mut res = state
        .db
        .query(
            "SELECT *, <string>fetched_at AS fetched_at FROM link_preview WHERE url = $url LIMIT 1",
        )
        .bind(("url", url.clone()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cached: Option<LinkPreview> = res.take(0).unwrap_or(None);
    if let Some(lp) = cached {
        return Ok(Json(ApiLinkPreview::from(lp)));
    }

    // Fetch the URL
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (compatible; IlmBot/1.0; +https://ilm.app)",
        )
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("Link preview fetch failed for {url}: {e}");
            StatusCode::BAD_GATEWAY
        })?;

    let html = resp.text().await.map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Extract OG tags via regex
    let extract = |pattern: &str| -> Option<String> {
        regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(&html))
            .and_then(|caps| caps.get(1))
            .map(|m| html_escape_decode(m.as_str()))
    };

    // Handle both attribute orders: property before content AND content before property
    let extract_og = |prop: &str| -> Option<String> {
        let p1 = format!(r#"<meta[^>]+property="{prop}"[^>]+content="([^"]*)""#);
        let p2 = format!(r#"<meta[^>]+content="([^"]*)"[^>]+property="{prop}""#);
        extract(&p1).or_else(|| extract(&p2))
    };

    let og_title = extract_og("og:title");
    let og_desc = extract_og("og:description");
    let og_image = extract_og("og:image");
    let html_title = extract(r#"<title[^>]*>([^<]*)</title>"#);

    let title = og_title.or(html_title);
    let domain = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|s| s.split('/').next())
        .map(|s| s.to_string());

    // Cache the result (delete old + create new to handle duplicates)
    let now = crate::web::note_handlers::now_iso();
    let _ = state
        .db
        .query(
            "DELETE link_preview WHERE url = $url; \
             CREATE link_preview CONTENT {
                url: $url, title: $title, description: $desc,
                image: $image, domain: $domain, fetched_at: $now
            }",
        )
        .bind(("url", url.clone()))
        .bind(("title", title.clone()))
        .bind(("desc", og_desc.clone()))
        .bind(("image", og_image.clone()))
        .bind(("domain", domain.clone()))
        .bind(("now", now))
        .await;

    Ok(Json(ApiLinkPreview {
        url,
        title,
        description: og_desc,
        image: og_image,
        domain,
    }))
}

fn html_escape_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
}
