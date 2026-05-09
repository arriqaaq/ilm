use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::llm::ChatOptions;
use crate::models::{ApiHadith, ApiHadithSearchResult};
use crate::quran::models::{
    ApiAyahSearchResult, ApiPhraseWithAyahs, ApiQuranWord, ApiReciter, ApiSimilarAyah, ApiSurah,
    AyahSimilarResponse, QuranPhrase, QuranSearchResponse, QuranStatsResponse, RootSearchResponse,
    SurahDetailResponse,
};

use super::AppState;
use super::sse::token_stream_to_sse;

// ── Query parameter types ──

#[derive(Deserialize, IntoParams)]
pub struct AyahHadithParams {
    /// When true, also return semantically-related hadiths in addition to the
    /// curated `references_hadith` edges from Quran.com.
    pub include_semantic: Option<bool>,
    pub semantic_limit: Option<usize>,
}

#[derive(Deserialize, IntoParams)]
pub struct QuranSearchParams {
    pub q: Option<String>,
    /// Search mode: `text` (BM25, default), `semantic`, or `hybrid`.
    #[serde(rename = "type")]
    #[param(rename = "type")]
    pub search_type: Option<String>,
    pub limit: Option<usize>,
    pub page: Option<usize>,
}

#[derive(Deserialize, IntoParams)]
pub struct QuranBrowseParams {
    /// Limit results to a specific surah (1-114).
    pub surah: Option<i64>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Deserialize, ToSchema)]
pub struct QuranAskRequest {
    pub question: String,
    /// Optional model override.
    pub model: Option<String>,
}

// ── Handlers ──

/// Quran-wide counts (number of surahs and ayahs).
#[utoipa::path(
    get,
    path = "/quran/meta",
    tag = "Quran",
    responses((status = 200, body = QuranStatsResponse))
)]
pub async fn quran_stats(State(state): State<AppState>) -> impl IntoResponse {
    match crate::services::quran::stats(&state).await {
        Ok(s) => Json(s),
        Err(e) => {
            tracing::error!("Quran stats failed: {e}");
            Json(QuranStatsResponse {
                surah_count: 0,
                ayah_count: 0,
            })
        }
    }
}

use surrealdb::types::SurrealValue;

/// All reciters available for ayah-level audio playback.
#[utoipa::path(
    get,
    path = "/quran/reciters",
    tag = "Quran",
    responses((status = 200, body = Vec<ApiReciter>))
)]
pub async fn reciters(State(state): State<AppState>) -> impl IntoResponse {
    match crate::services::quran::list_reciters(&state).await {
        Ok(rs) => Json(rs),
        Err(e) => {
            tracing::error!("Reciters query failed: {e}");
            Json(Vec::<ApiReciter>::new())
        }
    }
}

/// All 114 surahs with metadata (name, ayah count, revelation type).
#[utoipa::path(
    get,
    path = "/quran/surahs",
    tag = "Quran",
    responses((status = 200, body = Vec<ApiSurah>))
)]
pub async fn surah_list(State(state): State<AppState>) -> impl IntoResponse {
    match crate::services::quran::list_surahs(&state).await {
        Ok(s) => Json(s),
        Err(e) => {
            tracing::error!("Surah list query failed: {e}");
            Json(Vec::<ApiSurah>::new())
        }
    }
}

/// One surah plus all its ayahs (Arabic text, English translation, juz/hizb markers).
#[utoipa::path(
    get,
    path = "/quran/surahs/{number}",
    tag = "Quran",
    params(("number" = i64, Path, description = "Surah number 1-114")),
    responses((status = 200, body = SurahDetailResponse))
)]
pub async fn surah_detail(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<Json<SurahDetailResponse>, StatusCode> {
    match crate::services::quran::get_surah(&state, number).await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) if crate::services::quran::is_not_found(&e) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Surah detail query failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Search Quran ayahs by free-text query.
///
/// `type=text` (default) uses BM25 over Arabic + lemmatized Arabic + English;
/// `type=semantic` uses vector similarity; `type=hybrid` fuses both.
#[utoipa::path(
    get,
    path = "/search/quran",
    tag = "Search",
    params(QuranSearchParams),
    responses((status = 200, body = QuranSearchResponse))
)]
pub async fn quran_search(
    State(state): State<AppState>,
    Query(params): Query<QuranSearchParams>,
) -> Result<Json<QuranSearchResponse>, StatusCode> {
    let query = params.q.unwrap_or_default();
    if query.trim().is_empty() {
        return Ok(Json(QuranSearchResponse {
            query,
            search_type: "text".into(),
            ayahs: vec![],
            page: 1,
            limit: params.limit.unwrap_or(20),
            has_more: false,
        }));
    }

    let limit = params.limit.unwrap_or(20);
    let page = params.page.unwrap_or(1);
    let offset = (page - 1) * limit;
    let search_type = params.search_type.as_deref().unwrap_or("text");

    let results = match (search_type, state.embedder.as_deref()) {
        ("semantic", Some(embedder)) => {
            crate::quran::search::search_ayahs_semantic(&state.db, embedder, &query, limit, offset)
                .await
        }
        ("hybrid", Some(embedder)) => {
            crate::quran::search::search_ayahs_hybrid(&state.db, embedder, &query, limit, offset)
                .await
        }
        ("semantic" | "hybrid", None) => {
            // Advanced features disabled — fall back to text search
            crate::quran::search::search_ayahs_text(&state.db, &query, limit, offset).await
        }
        _ => crate::quran::search::search_ayahs_text(&state.db, &query, limit, offset).await,
    };

    let ayahs = results.map_err(|e| {
        tracing::error!("Quran search failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let has_more = ayahs.len() == limit;

    Ok(Json(QuranSearchResponse {
        query,
        search_type: search_type.to_string(),
        ayahs: ayahs.into_iter().map(ApiAyahSearchResult::from).collect(),
        page,
        limit,
        has_more,
    }))
}

/// Paginated browse over ayahs, optionally filtered by surah.
#[utoipa::path(
    get,
    path = "/quran/ayahs",
    tag = "Quran",
    params(QuranBrowseParams),
    responses((status = 200, description = "{ data: ApiAyah[], page, limit, has_more, total? }", body = serde_json::Value))
)]
pub async fn ayah_browse(
    State(state): State<AppState>,
    Query(params): Query<QuranBrowseParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50);
    let page = params.page.unwrap_or(1);
    match crate::services::quran::browse_ayahs(&state, params.surah, page, limit).await {
        Ok(resp) => Json(serde_json::to_value(resp).unwrap()),
        Err(e) => {
            tracing::error!("Ayah browse query failed: {e}");
            Json(serde_json::json!({
                "data": [], "page": page, "limit": limit, "has_more": false,
            }))
        }
    }
}

/// Quran-grounded GraphRAG question answering.
///
/// **Streaming response** (SSE). First event carries `{quran_sources}`;
/// subsequent events stream LLM tokens. Rate-limited harder than read endpoints.
#[utoipa::path(
    post,
    path = "/ask/quran",
    tag = "Ask",
    request_body = QuranAskRequest,
    responses((status = 200, description = "SSE token stream with sources prefix", body = serde_json::Value, content_type = "text/event-stream"))
)]
pub async fn ask_quran(
    State(state): State<AppState>,
    Json(body): Json<QuranAskRequest>,
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
        crate::services::ask::AskScope::Quran,
    )
    .await
    .map_err(|e| {
        tracing::error!("Agentic RAG ask (quran) failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    use crate::services::ask::AgenticResult;

    let (sources_event, token_stream) = match result {
        AgenticResult::Semantic {
            ayah_sources,
            token_stream,
            ..
        } => {
            let ayah_api: Vec<ApiAyahSearchResult> = ayah_sources
                .into_iter()
                .map(ApiAyahSearchResult::from)
                .collect();
            let event = format!(
                "data: {}\n\n",
                serde_json::to_string(&serde_json::json!({ "quran_sources": ayah_api })).unwrap()
            );
            (event, token_stream)
        }
        // Structured intents don't fire under Quran scope (classifier is
        // skipped). If that ever changes, surface whatever narrator/hadith
        // data came back so callers see something meaningful.
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
    };

    let sse_stream =
        futures::stream::once(
            async move { Ok::<_, std::io::Error>(bytes::Bytes::from(sources_event)) },
        )
        .chain(token_stream_to_sse(token_stream));

    Ok(Response::builder()
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .body(Body::from_stream(sse_stream))
        .unwrap())
}

// ── Ayah-Hadith reference handlers ──

#[derive(serde::Serialize, ToSchema)]
pub struct AyahHadithResponse {
    /// Curated hadith references for this ayah, sourced from Quran.com.
    pub curated: Vec<ApiHadith>,
    /// Semantically-related hadiths via vector search; only populated when
    /// `?include_semantic=true` is passed.
    pub related: Option<Vec<ApiHadithSearchResult>>,
}

/// Hadiths that reference this ayah.
///
/// `curated` are the explicit `references_hadith` edges sourced from
/// Quran.com. Set `?include_semantic=true` to additionally surface
/// semantically-related hadiths via vector search.
#[utoipa::path(
    get,
    path = "/quran/ayahs/{surah}/{ayah}/hadiths",
    tag = "Quran",
    params(
        ("surah" = i64, Path),
        ("ayah" = i64, Path),
        AyahHadithParams,
    ),
    responses((status = 200, body = AyahHadithResponse))
)]
pub async fn ayah_hadiths(
    State(state): State<AppState>,
    Path((surah, ayah)): Path<(i64, i64)>,
    Query(params): Query<AyahHadithParams>,
) -> Result<Json<AyahHadithResponse>, StatusCode> {
    let include_semantic = params.include_semantic.unwrap_or(false);
    let semantic_limit = params.semantic_limit.unwrap_or(5);
    match crate::services::quran::get_ayah_hadiths(
        &state,
        surah,
        ayah,
        include_semantic,
        semantic_limit,
    )
    .await
    {
        Ok(resp) => Ok(Json(AyahHadithResponse {
            curated: resp.curated,
            related: resp.related,
        })),
        Err(e) => {
            tracing::error!("Ayah-hadith lookup failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Per-ayah curated hadith counts for a surah. Map key = ayah number (string).
#[utoipa::path(
    get,
    path = "/quran/surahs/{number}/hadith-counts",
    tag = "Quran",
    params(("number" = i64, Path, description = "Surah number 1-114")),
    responses((status = 200, body = std::collections::HashMap<String, i64>))
)]
pub async fn surah_hadith_counts(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<Json<std::collections::HashMap<String, i64>>, StatusCode> {
    crate::services::quran::get_surah_hadith_counts(&state, number)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get hadith counts: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Per-ayah counts of `similar_to` + `shares_phrase` edges in this surah —
/// used by the Quran reader to mark ayahs with mutashabihat connections.
#[utoipa::path(
    get,
    path = "/quran/surahs/{number}/similar-counts",
    tag = "Quran",
    params(("number" = i64, Path, description = "Surah number 1-114")),
    responses((status = 200, body = std::collections::HashMap<String, i64>))
)]
pub async fn surah_similar_counts(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<Json<std::collections::HashMap<String, i64>>, StatusCode> {
    crate::services::quran::get_surah_similar_counts(&state, number)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Surah similar counts failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

#[allow(dead_code)]
fn parse_ayah_key(key: &str) -> Option<(i64, i64)> {
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() == 2 {
        let s = parts[0].parse().ok()?;
        let a = parts[1].parse().ok()?;
        Some((s, a))
    } else {
        None
    }
}

// ── Word Morphology Handlers ──

/// Word-by-word morphology for one ayah (root, lemma, POS, transliteration,
/// English gloss). Sourced from corpus.quran.com + QUL.
#[utoipa::path(
    get,
    path = "/quran/ayahs/{surah}/{ayah}/words",
    tag = "Quran",
    params(
        ("surah" = i64, Path, description = "Surah number (1-114)"),
        ("ayah" = i64, Path, description = "Ayah number within the surah")
    ),
    responses((status = 200, body = Vec<ApiQuranWord>))
)]
pub async fn ayah_words(
    State(state): State<AppState>,
    Path((surah, ayah)): Path<(i64, i64)>,
) -> Result<Json<Vec<ApiQuranWord>>, (StatusCode, String)> {
    crate::services::quran::get_ayah_words(&state, surah, ayah)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Concordance of every Quran occurrence of an Arabic root.
#[utoipa::path(
    get,
    path = "/quran/roots/{root}",
    tag = "Quran",
    params(("root" = String, Path, description = "Arabic root, e.g. `ك ت ب` (URL-encoded)")),
    responses((status = 200, body = RootSearchResponse))
)]
pub async fn root_search(
    State(state): State<AppState>,
    Path(root): Path<String>,
) -> Result<Json<RootSearchResponse>, (StatusCode, String)> {
    crate::services::quran::search_by_root(&state, &root)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ── Similar Ayahs / Mutashabihat Handlers ──

use crate::models::record_id_key_string;
use surrealdb::types::RecordId;

#[derive(Debug, SurrealValue)]
struct SimilarEdgeRow {
    score: i64,
    coverage: i64,
    matched_positions: Option<String>,
    surah_number: i64,
    ayah_number: i64,
    text_ar: Option<String>,
    text_en: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct PhraseEdgeRow {
    id: Option<RecordId>,
    text_ar: String,
    occurrence: i64,
    verses_count: i64,
    chapters_count: i64,
}

#[derive(Debug, SurrealValue)]
struct AyahKeyRow {
    surah_number: i64,
    ayah_number: i64,
}

/// Similar ayahs (mutashabihat) and shared phrases for one ayah.
///
/// `similar` is a ranked list of other ayahs that share enough wording to be
/// considered parallels; `phrases` lists the shared phrase clusters this ayah
/// belongs to (each phrase points back to all ayahs containing it).
#[utoipa::path(
    get,
    path = "/quran/ayahs/{surah}/{ayah}/similar",
    tag = "Quran",
    params(
        ("surah" = i64, Path),
        ("ayah" = i64, Path)
    ),
    responses((status = 200, body = AyahSimilarResponse))
)]
pub async fn ayah_similar(
    State(state): State<AppState>,
    Path((surah, ayah)): Path<(i64, i64)>,
) -> Result<Json<AyahSimilarResponse>, (StatusCode, String)> {
    let ayah_id = RecordId::new("ayah", format!("{surah}_{ayah}"));

    // 1. Query similar ayahs
    let mut res = state
        .db
        .query(
            "SELECT score, coverage, matched_positions, \
             out.surah_number AS surah_number, out.ayah_number AS ayah_number, \
             out.text_ar AS text_ar, out.text_en AS text_en \
             FROM similar_to WHERE in = $ayah_id ORDER BY score DESC LIMIT 20",
        )
        .bind(("ayah_id", ayah_id.clone()))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let similar_rows: Vec<SimilarEdgeRow> = res
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let similar: Vec<ApiSimilarAyah> = similar_rows
        .into_iter()
        .map(|r| ApiSimilarAyah {
            ayah_key: format!("{}:{}", r.surah_number, r.ayah_number),
            score: r.score,
            coverage: r.coverage,
            matched_positions: r
                .matched_positions
                .and_then(|s| serde_json::from_str(&s).ok()),
            text_ar: r.text_ar,
            text_en: r.text_en,
        })
        .collect();

    // 2. Query shared phrases for this ayah
    let mut res2 = state
        .db
        .query(
            "SELECT out.id AS id, out.text_ar AS text_ar, out.occurrence AS occurrence, \
             out.verses_count AS verses_count, out.chapters_count AS chapters_count \
             FROM shares_phrase WHERE in = $ayah_id",
        )
        .bind(("ayah_id", ayah_id.clone()))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let phrase_rows: Vec<PhraseEdgeRow> = res2
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Batch-fetch all ayahs sharing any of these phrases (single query instead of N+1)
    let phrase_ids: Vec<RecordId> = phrase_rows.iter().filter_map(|p| p.id.clone()).collect();

    let mut phrase_ayah_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    if !phrase_ids.is_empty() {
        #[derive(Debug, SurrealValue)]
        struct PhraseAyahRow {
            phrase_id: Option<RecordId>,
            surah_number: i64,
            ayah_number: i64,
        }

        let mut res3 = state
            .db
            .query(
                "SELECT out AS phrase_id, in.surah_number AS surah_number, in.ayah_number AS ayah_number \
                 FROM shares_phrase WHERE out IN $phrase_ids AND in != $ayah_id",
            )
            .bind(("phrase_ids", phrase_ids))
            .bind(("ayah_id", ayah_id.clone()))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rows: Vec<PhraseAyahRow> = res3
            .take(0)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for row in rows {
            if let Some(ref pid) = row.phrase_id {
                let key = record_id_key_string(pid);
                phrase_ayah_map
                    .entry(key)
                    .or_default()
                    .push(format!("{}:{}", row.surah_number, row.ayah_number));
            }
        }
    }

    let phrases: Vec<ApiPhraseWithAyahs> = phrase_rows
        .into_iter()
        .filter_map(|p| {
            let pid = p.id.as_ref()?;
            let pid_str = record_id_key_string(pid);
            Some(ApiPhraseWithAyahs {
                id: pid_str.clone(),
                text_ar: p.text_ar,
                occurrence: p.occurrence,
                ayah_keys: phrase_ayah_map.remove(&pid_str).unwrap_or_default(),
            })
        })
        .collect();

    Ok(Json(AyahSimilarResponse { similar, phrases }))
}

/// One shared-phrase cluster — the Arabic text of the phrase plus every ayah
/// that contains it.
#[utoipa::path(
    get,
    path = "/quran/phrases/{id}",
    tag = "Quran",
    params(("id" = String, Path)),
    responses(
        (status = 200, body = ApiPhraseWithAyahs),
        (status = 404, description = "Phrase not found")
    )
)]
pub async fn phrase_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiPhraseWithAyahs>, (StatusCode, String)> {
    let phrase_id = RecordId::new("quran_phrase", id.clone());

    // Get phrase record
    let mut res = state
        .db
        .query("SELECT * FROM $phrase_id")
        .bind(("phrase_id", phrase_id.clone()))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let phrase: Option<QuranPhrase> = res
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let phrase = phrase.ok_or_else(|| (StatusCode::NOT_FOUND, format!("Phrase {id} not found")))?;

    // Get all ayahs sharing this phrase
    let mut res2 = state
        .db
        .query(
            "SELECT in.surah_number AS surah_number, in.ayah_number AS ayah_number \
             FROM shares_phrase WHERE out = $phrase_id",
        )
        .bind(("phrase_id", phrase_id))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ayah_rows: Vec<AyahKeyRow> = res2
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ayah_keys: Vec<String> = ayah_rows
        .into_iter()
        .map(|r| format!("{}:{}", r.surah_number, r.ayah_number))
        .collect();

    Ok(Json(ApiPhraseWithAyahs {
        id: phrase.id.as_ref().map(record_id_key_string).unwrap_or(id),
        text_ar: phrase.text_ar,
        occurrence: phrase.occurrence,
        ayah_keys,
    }))
}
