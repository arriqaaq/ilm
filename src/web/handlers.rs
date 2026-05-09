use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use serde::Deserialize;
use std::collections::HashSet;
use surrealdb::types::{RecordId, SurrealValue};

use super::sse::token_stream_to_sse;
use crate::analysis;
use crate::llm::ChatOptions;
use crate::models::{
    ApiCollection, ApiHadith, ApiHadithFamily, ApiHadithSearchResult, ApiNarrator,
    ApiNarratorSearchResult, ApiNarratorWithCount, Collection, CommonNarratorsResponse, GraphData,
    GraphEdge, GraphEdgeData, GraphNode, GraphNodeData, HADITH_FIELDS, HADITH_SEARCH_FIELDS,
    Hadith, HadithFamily, HadithSearchResult, IsnadSearchResponse, Narrator, PaginatedResponse,
    StatsResponse, record_id_key_string, record_id_string,
};

use super::AppState;

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

pub async fn app_config(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "advanced_enabled": state.advanced_enabled,
        "llm_available": state.llm.is_some(),
        "llm_provider": state.llm.as_ref().map(|l| l.provider_name()),
        "embed_available": state.embedder.is_some(),
        "embed_provider": state.embedder.as_ref().map(|e| e.provider_name()),
        "reranker_available": state.reranker.is_some(),
    }))
}

// ── Query parameter types ──

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    #[serde(rename = "type")]
    pub search_type: Option<String>,
    pub limit: Option<usize>,
    pub page: Option<usize>,
    /// Opt-in cross-encoder reranking. Only honoured for `type=hybrid` AND
    /// when the server was started with `--reranker`. Otherwise ignored.
    pub rerank: Option<bool>,
}

#[derive(Deserialize)]
pub struct ListParams {
    pub book: Option<i64>,
    pub number: Option<i64>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub q: Option<String>,
    pub generation: Option<String>,
}

#[derive(Deserialize)]
pub struct AskRequest {
    pub question: String,
    pub model: Option<String>,
}

#[derive(Deserialize)]
pub struct AutocompleteParams {
    pub q: String,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct IsnadSearchRequest {
    pub narrator_ids: Vec<String>,
    pub mode: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct CommonNarratorsParams {
    pub a: String,
    pub b: String,
}

// ── API Handlers ──

pub async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    let result = state
        .db
        .query(
            "SELECT count() AS c FROM hadith GROUP ALL; \
             SELECT count() AS c FROM narrator GROUP ALL; \
             SELECT count() AS c FROM collection GROUP ALL;",
        )
        .await;

    let (hadith_count, narrator_count, book_count) = match result {
        Ok(mut res) => {
            let h: Option<CountResult> = res.take(0).unwrap_or(None);
            let n: Option<CountResult> = res.take(1).unwrap_or(None);
            let b: Option<CountResult> = res.take(2).unwrap_or(None);
            (
                h.map(|r| r.c).unwrap_or(0),
                n.map(|r| r.c).unwrap_or(0),
                b.map(|r| r.c).unwrap_or(0),
            )
        }
        Err(e) => {
            tracing::error!("Stats query failed: {e}");
            (0, 0, 0)
        }
    };

    Json(StatsResponse {
        hadith_count,
        narrator_count,
        book_count,
    })
}

pub async fn books(State(state): State<AppState>) -> impl IntoResponse {
    let books: Vec<Collection> = match state
        .db
        .query("SELECT * FROM collection ORDER BY collection_id ASC")
        .await
    {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Books query failed: {e}");
            vec![]
        }
    };

    Json(
        books
            .into_iter()
            .map(ApiCollection::from)
            .collect::<Vec<_>>(),
    )
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    let search_type = params.search_type.unwrap_or_else(|| "hybrid".into());
    let limit = params.limit.unwrap_or(20);
    let rerank = params.rerank.unwrap_or(false);

    if query.is_empty() {
        return Json(serde_json::json!({
            "query": query,
            "search_type": search_type,
            "hadiths": [],
            "narrators": []
        }));
    }

    let hadith_results = match (search_type.as_str(), state.embedder.as_deref()) {
        ("semantic", Some(embedder)) => {
            crate::search::search_hadiths_semantic(&state.db, embedder, &query, limit)
                .await
                .unwrap_or_default()
        }
        ("semantic" | "hybrid", None) => {
            // Advanced features disabled — fall back to text search
            crate::search::search_hadiths_text(&state.db, &query, limit, 0)
                .await
                .unwrap_or_default()
        }
        ("text", _) => crate::search::search_hadiths_text(&state.db, &query, limit, 0)
            .await
            .unwrap_or_default(),
        (_, Some(embedder)) => {
            let reranker = if rerank {
                state.reranker.as_deref()
            } else {
                None
            };
            crate::search::search_hadiths_hybrid(&state.db, embedder, &query, limit, 0, reranker)
                .await
                .unwrap_or_default()
        }
        _ => unreachable!(),
    };

    let narrator_results = crate::search::search_narrators(&state.db, &query, 10, 0)
        .await
        .unwrap_or_default();

    Json(serde_json::json!({
        "query": query,
        "search_type": search_type,
        "hadiths": hadith_results.into_iter().map(ApiHadithSearchResult::from).collect::<Vec<_>>(),
        "narrators": narrator_results.into_iter().map(ApiNarratorSearchResult::from).collect::<Vec<_>>()
    }))
}

pub async fn hadith_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = (page - 1) * limit;

    let mut conditions: Vec<String> = Vec::new();
    if let Some(book_id) = params.book {
        conditions.push(format!("collection_id = {book_id}"));
    }
    if let Some(number) = params.number {
        conditions.push(format!("hadith_number = {number}"));
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };
    let query = format!(
        "SELECT {HADITH_FIELDS} FROM hadith {where_clause} \
         ORDER BY hadith_number ASC LIMIT {limit} START {offset}"
    );

    let hadiths: Vec<Hadith> = match state.db.query(&query).await {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Hadith list query failed: {e}");
            vec![]
        }
    };
    let has_more = hadiths.len() == limit;

    Json(PaginatedResponse {
        data: hadiths.into_iter().map(ApiHadith::from).collect(),
        page,
        has_more,
    })
}

pub async fn hadith_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let hrid = rid("hadith", &id);

    // Single multi-statement query instead of 4 sequential round trips
    let mut res = state
        .db
        .query(format!(
            "SELECT {HADITH_FIELDS} FROM $rid; \
             SELECT <-narrates<-narrator.* AS narrators FROM $rid; \
             SELECT in.id AS id, in.surah_number AS surah_number, in.ayah_number AS ayah_number, \
               in.text_ar AS text_ar, in.text_en AS text_en, in.tafsir_en AS tafsir_en \
               FROM references_hadith WHERE out = $rid ORDER BY surah_number, ayah_number; \
             SELECT ->similar_to->hadith.{{{HADITH_FIELDS}}} AS hadiths FROM $rid;"
        ))
        .bind(("rid", hrid))
        .await
        .map_err(|e| {
            tracing::error!("Hadith detail query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let hadith: Option<Hadith> = res.take(0).unwrap_or(None);
    let hadith = hadith.ok_or(StatusCode::NOT_FOUND)?;

    let narrators: Vec<Narrator> = {
        let result: Option<NarratorsResult> = res.take(1).unwrap_or(None);
        result.map(|r| r.narrators).unwrap_or_default()
    };

    let linked_ayahs: Vec<crate::quran::models::ApiAyah> = {
        let ayahs: Vec<crate::quran::models::Ayah> = res.take(2).unwrap_or_default();
        ayahs
            .into_iter()
            .map(crate::quran::models::ApiAyah::from)
            .collect()
    };

    let similar_hadiths: Vec<ApiHadith> = {
        #[derive(Debug, SurrealValue)]
        struct HadithsResult {
            hadiths: Vec<Hadith>,
        }
        let result: Option<HadithsResult> = res.take(3).unwrap_or(None);
        result
            .map(|r| r.hadiths.into_iter().map(ApiHadith::from).collect())
            .unwrap_or_default()
    };

    Ok(Json(serde_json::json!({
        "hadith": ApiHadith::from(hadith),
        "narrators": narrators.into_iter().map(ApiNarrator::from).collect::<Vec<_>>(),
        "linked_ayahs": linked_ayahs,
        "similar_hadiths": similar_hadiths
    })))
}

pub async fn narrator_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(50);
    let offset = (page - 1) * limit;

    // Build WHERE clauses dynamically
    let mut conditions: Vec<String> = Vec::new();
    if let Some(q) = &params.q {
        let _ = q; // used via bind
        conditions.push(
            "(string::lowercase(name_en) CONTAINS string::lowercase($q) OR name_ar CONTAINS $q)"
                .to_string(),
        );
    }
    if let Some(generation) = &params.generation {
        let _ = generation;
        conditions.push("generation = $generation".to_string());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let query_str = format!(
        "SELECT * FROM narrator {where_clause} ORDER BY hadith_count DESC LIMIT $limit START $offset"
    );

    let mut query = state.db.query(&query_str);
    if let Some(q) = &params.q {
        query = query.bind(("q", q.clone()));
    }
    if let Some(generation) = &params.generation {
        query = query.bind(("generation", generation.clone()));
    }
    query = query.bind(("limit", limit)).bind(("offset", offset));

    let narrators: Vec<NarratorWithCount> = match query.await {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Narrator list query failed: {e}");
            vec![]
        }
    };

    let has_more = narrators.len() == limit;
    let api_narrators: Vec<ApiNarratorWithCount> = narrators
        .into_iter()
        .map(|n| ApiNarratorWithCount {
            id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n.name_ar,
            name_en: n.name_en,
            generation: n.generation,
            bio: n.bio,
            kunya: n.kunya,
            death_year: n.death_year,
            hadith_count: n.hadith_count.unwrap_or(0),
        })
        .collect();

    Json(PaginatedResponse {
        data: api_narrators,
        page,
        has_more,
    })
}

pub async fn narrator_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let nrid = rid("narrator", &id);

    // Single multi-statement query instead of 4 sequential round trips
    let (narrator, hadiths, teachers, students) = match state
        .db
        .query(
            format!(
                "SELECT * FROM $rid; \
                 SELECT ->narrates->hadith.{{{HADITH_FIELDS}}} AS hadiths FROM $rid; \
                 SELECT array::distinct(array::filter(->heard_from->narrator.*, |$v| $v IS NOT NONE)) AS teachers FROM $rid; \
                 SELECT array::distinct(array::filter(<-heard_from<-narrator.*, |$v| $v IS NOT NONE)) AS students FROM $rid;"
            ),
        )
        .bind(("rid", nrid))
        .await
    {
        Ok(mut res) => {
            let narrator: Option<Narrator> = res.take(0).unwrap_or(None);
            let hadiths_result: Option<HadithsResult> = res.take(1).unwrap_or(None);
            let teachers_result: Option<TeachersResult> = res.take(2).unwrap_or(None);
            let students_result: Option<StudentsResult> = res.take(3).unwrap_or(None);
            (
                narrator,
                hadiths_result.map(|r| r.hadiths).unwrap_or_default(),
                teachers_result.map(|r| r.teachers).unwrap_or_default(),
                students_result.map(|r| r.students).unwrap_or_default(),
            )
        }
        Err(e) => {
            tracing::error!("Narrator detail query failed: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let narrator = narrator.ok_or(StatusCode::NOT_FOUND)?;

    // Deduplicate by narrator ID
    let dedup_narrators = |narrators: Vec<Narrator>| -> Vec<Narrator> {
        let mut seen = HashSet::new();
        narrators
            .into_iter()
            .filter(|n| {
                n.id.as_ref()
                    .map(|id| seen.insert(record_id_string(id)))
                    .unwrap_or(false)
            })
            .collect()
    };
    let teachers = dedup_narrators(teachers);
    let students = dedup_narrators(students);

    Ok(Json(serde_json::json!({
        "narrator": ApiNarrator::from(narrator),
        "hadiths": hadiths.into_iter().map(ApiHadith::from).collect::<Vec<_>>(),
        "teachers": teachers.into_iter().map(ApiNarrator::from).collect::<Vec<_>>(),
        "students": students.into_iter().map(ApiNarrator::from).collect::<Vec<_>>()
    })))
}

pub async fn chain_graph_data(
    State(state): State<AppState>,
    Path(hadith_id): Path<String>,
) -> impl IntoResponse {
    let hrid = rid("hadith", &hadith_id);

    let narrators = match state
        .db
        .query("SELECT <-narrates<-narrator.* AS narrators FROM $rid")
        .bind(("rid", hrid.clone()))
        .await
    {
        Ok(mut r) => {
            let result: Option<NarratorsResult> = r.take(0).unwrap_or(None);
            result.map(|r| r.narrators).unwrap_or_default()
        }
        Err(e) => {
            tracing::error!("Chain narrators query failed: {e}");
            vec![]
        }
    };

    let edges: Vec<HeardFromEdge> = match state
        .db
        .query("SELECT in AS in_id, out AS out_id, chain_position FROM heard_from WHERE hadith_ref = $rid ORDER BY chain_position")
        .bind(("rid", hrid))
        .await
    {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Chain edges query failed: {e}");
            vec![]
        }
    };

    let mut graph = GraphData {
        nodes: Vec::new(),
        edges: Vec::new(),
        total_teachers: None,
        total_students: None,
    };

    for narrator in &narrators {
        if let Some(id) = &narrator.id {
            graph.nodes.push(GraphNode {
                data: GraphNodeData {
                    id: record_id_string(id),
                    label: narrator
                        .name_ar
                        .clone()
                        .unwrap_or_else(|| narrator.name_en.clone()),
                    label_en: narrator.name_en.clone(),
                    node_type: "narrator".into(),
                    generation: narrator.generation.clone(),
                },
            });
        }
    }

    for (i, edge) in edges.iter().enumerate() {
        graph.edges.push(GraphEdge {
            data: GraphEdgeData {
                id: format!("e{i}"),
                source: record_id_string(&edge.in_id),
                target: record_id_string(&edge.out_id),
                label: "heard from".into(),
                chain_position: edge.chain_position,
            },
        });
    }

    Json(graph)
}

pub async fn narrator_graph_data(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let nrid = rid("narrator", &id);

    let (narrator, teachers, students) = match state
        .db
        .query(
            "SELECT * FROM $rid; \
             SELECT array::distinct(array::filter(->heard_from->narrator.*, |$v| $v IS NOT NONE)) AS teachers FROM $rid; \
             SELECT array::distinct(array::filter(<-heard_from<-narrator.*, |$v| $v IS NOT NONE)) AS students FROM $rid;",
        )
        .bind(("rid", nrid))
        .await
    {
        Ok(mut res) => {
            let narrator: Option<Narrator> = res.take(0).unwrap_or(None);
            let teachers_result: Option<TeachersResult> = res.take(1).unwrap_or(None);
            let students_result: Option<StudentsResult> = res.take(2).unwrap_or(None);
            (
                narrator,
                teachers_result.map(|r| r.teachers).unwrap_or_default(),
                students_result.map(|r| r.students).unwrap_or_default(),
            )
        }
        Err(e) => {
            tracing::error!("Narrator graph query failed: {e}");
            (None, vec![], vec![])
        }
    };

    // Deduplicate by narrator ID
    let dedup = |narrators: Vec<Narrator>| -> Vec<Narrator> {
        let mut seen = HashSet::new();
        narrators
            .into_iter()
            .filter(|n| {
                n.id.as_ref()
                    .map(|id| seen.insert(record_id_string(id)))
                    .unwrap_or(false)
            })
            .collect()
    };
    let teachers = dedup(teachers);
    let students = dedup(students);
    let total_teachers = teachers.len();
    let total_students = students.len();

    // Cap for graph rendering performance
    const MAX_GRAPH_NODES: usize = 25;
    let teachers: Vec<_> = teachers.into_iter().take(MAX_GRAPH_NODES).collect();
    let students: Vec<_> = students.into_iter().take(MAX_GRAPH_NODES).collect();

    let mut graph = GraphData {
        nodes: Vec::new(),
        edges: Vec::new(),
        total_teachers: Some(total_teachers),
        total_students: Some(total_students),
    };

    if let Some(narrator) = &narrator
        && let Some(nid) = &narrator.id
    {
        let nid_str = record_id_string(nid);
        graph.nodes.push(GraphNode {
            data: GraphNodeData {
                id: nid_str.clone(),
                label: narrator
                    .name_ar
                    .clone()
                    .unwrap_or_else(|| narrator.name_en.clone()),
                label_en: narrator.name_en.clone(),
                node_type: "center".into(),
                generation: narrator.generation.clone(),
            },
        });

        for (i, teacher) in teachers.iter().enumerate() {
            if let Some(tid) = &teacher.id {
                let tid_str = record_id_string(tid);
                graph.nodes.push(GraphNode {
                    data: GraphNodeData {
                        id: tid_str.clone(),
                        label: teacher
                            .name_ar
                            .clone()
                            .unwrap_or_else(|| teacher.name_en.clone()),
                        label_en: teacher.name_en.clone(),
                        node_type: "teacher".into(),
                        generation: teacher.generation.clone(),
                    },
                });
                graph.edges.push(GraphEdge {
                    data: GraphEdgeData {
                        id: format!("t{i}"),
                        source: nid_str.clone(),
                        target: tid_str,
                        label: "heard from".into(),
                        chain_position: None,
                    },
                });
            }
        }

        for (i, student) in students.iter().enumerate() {
            if let Some(sid) = &student.id {
                let sid_str = record_id_string(sid);
                graph.nodes.push(GraphNode {
                    data: GraphNodeData {
                        id: sid_str.clone(),
                        label: student
                            .name_ar
                            .clone()
                            .unwrap_or_else(|| student.name_en.clone()),
                        label_en: student.name_en.clone(),
                        node_type: "student".into(),
                        generation: student.generation.clone(),
                    },
                });
                graph.edges.push(GraphEdge {
                    data: GraphEdgeData {
                        id: format!("s{i}"),
                        source: sid_str,
                        target: nid_str.clone(),
                        label: "heard from".into(),
                        chain_position: None,
                    },
                });
            }
        }
    }

    Json(graph)
}

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

    let result = crate::agentic_rag::ask_agentic(
        llm.as_ref(),
        &state.db,
        embedder.as_ref(),
        &question,
        &opts,
        crate::agentic_rag::AskScope::Hadith,
    )
    .await
    .map_err(|e| {
        tracing::error!("Agentic RAG ask (hadith) failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    use crate::agentic_rag::AgenticResult;

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

pub async fn family_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = (page - 1) * limit;

    let mut res = state
        .db
        .query(
            "SELECT * FROM hadith_family ORDER BY variant_count DESC \
             LIMIT $limit START $offset",
        )
        .bind(("limit", limit + 1))
        .bind(("offset", offset))
        .await
        .unwrap();
    let families: Vec<HadithFamily> = res.take(0).unwrap_or_default();

    let has_more = families.len() > limit;
    let data: Vec<ApiHadithFamily> = families
        .into_iter()
        .take(limit)
        .map(ApiHadithFamily::from)
        .collect();

    Json(PaginatedResponse {
        data,
        page,
        has_more,
    })
}

pub async fn family_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let fid = rid("hadith_family", &id);

    let mut res = state
        .db
        .query(format!(
            "SELECT * FROM $fid; \
             SELECT {HADITH_FIELDS} FROM hadith WHERE family_id = $fid ORDER BY hadith_number ASC;"
        ))
        .bind(("fid", fid))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let family: Option<HadithFamily> =
        res.take(0).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let family = family.ok_or(StatusCode::NOT_FOUND)?;
    let hadiths: Vec<Hadith> = res.take(1).unwrap_or_default();

    Ok(Json(serde_json::json!({
        "family": ApiHadithFamily::from(family),
        "hadiths": hadiths.into_iter().map(ApiHadith::from).collect::<Vec<_>>(),
    })))
}

// ── Hadith gradings (multi-scholar verdicts) ──
//
// Returns one row per scholar per source book. For Bukhari/Muslim a synthetic
// "consensus sahih" row is prepended. The user explores narrator-level
// reliability by clicking through to each narrator's Tahdhib bio page — there
// is intentionally no automatic suspect-narrator surfacing here.

#[derive(Debug, SurrealValue)]
struct HadithGradingRow {
    scholar_key: String,
    scholar_ar: String,
    grade: String,
    grade_normalized: Option<String>,
    source_book_id: Option<i64>,
    source_page_index: Option<i64>,
    source_vol: Option<String>,
    source_page_num: Option<i64>,
    raw_text: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct CollectionIdRow {
    collection_id: i64,
}

pub async fn hadith_gradings(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let hrid = rid("hadith", &id);

    let collection_id: i64 = match state
        .db
        .query("SELECT collection_id FROM $rid")
        .bind(("rid", hrid.clone()))
        .await
    {
        Ok(mut r) => {
            let row: Option<CollectionIdRow> = r.take(0).unwrap_or(None);
            row.map(|r| r.collection_id).unwrap_or(0)
        }
        Err(e) => {
            tracing::warn!("hadith_gradings: collection lookup failed: {e}");
            0
        }
    };

    let stored: Vec<HadithGradingRow> = match state
        .db
        .query(
            "SELECT scholar_key, scholar_ar, grade, grade_normalized, \
             source_book_id, source_page_index, source_vol, source_page_num, \
             raw_text, notes FROM hadith_grading WHERE hadith_id = $rid",
        )
        .bind(("rid", hrid))
        .await
    {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::warn!("hadith_gradings: gradings query failed: {e}");
            vec![]
        }
    };

    let mut out: Vec<serde_json::Value> = Vec::with_capacity(stored.len() + 1);

    if collection_id == 1 || collection_id == 2 {
        let (key, ar) = if collection_id == 1 {
            ("bukhari", "البخاري")
        } else {
            ("muslim", "مسلم")
        };
        out.push(serde_json::json!({
            "scholar_key": key,
            "scholar_ar": ar,
            "grade": "صحيح",
            "grade_normalized": "sahih",
            "source_book_id": null,
            "source_page_index": null,
            "source_vol": null,
            "source_page_num": null,
            "raw_text": null,
            "notes": "consensus sahih",
        }));
    }

    for r in stored {
        out.push(serde_json::json!({
            "scholar_key": r.scholar_key,
            "scholar_ar": r.scholar_ar,
            "grade": r.grade,
            "grade_normalized": r.grade_normalized,
            "source_book_id": r.source_book_id,
            "source_page_index": r.source_page_index,
            "source_vol": r.source_vol,
            "source_page_num": r.source_page_num,
            "raw_text": r.raw_text,
            "notes": r.notes,
        }));
    }

    Json(serde_json::json!({
        "hadith_id": id,
        "gradings": out,
    }))
}

// ── Mustalah API handlers ──

pub async fn mustalah_stats(State(state): State<AppState>) -> impl IntoResponse {
    let mut res = state
        .db
        .query(
            "SELECT count() AS c FROM hadith_family GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis WHERE breadth_class = 'mutawatir' GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis WHERE breadth_class = 'mashhur' GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis WHERE breadth_class = 'aziz' GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis WHERE breadth_class = 'gharib' GROUP ALL",
        )
        .await
        .unwrap();

    let families: Option<CountResult> = res.take(0).unwrap_or(None);
    let analyzed: Option<CountResult> = res.take(1).unwrap_or(None);
    let mutawatir: Option<CountResult> = res.take(2).unwrap_or(None);
    let mashhur: Option<CountResult> = res.take(3).unwrap_or(None);
    let aziz: Option<CountResult> = res.take(4).unwrap_or(None);
    let gharib: Option<CountResult> = res.take(5).unwrap_or(None);

    Json(serde_json::json!({
        "family_count": families.map(|c| c.c).unwrap_or(0),
        "analyzed_count": analyzed.map(|c| c.c).unwrap_or(0),
        "mutawatir_count": mutawatir.map(|c| c.c).unwrap_or(0),
        "mashhur_count": mashhur.map(|c| c.c).unwrap_or(0),
        "aziz_count": aziz.map(|c| c.c).unwrap_or(0),
        "gharib_count": gharib.map(|c| c.c).unwrap_or(0),
    }))
}

pub async fn mustalah_family_analysis(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let fid = rid("hadith_family", &id);

    #[derive(Debug, SurrealValue, serde::Serialize)]
    struct IsnadRow {
        breadth_class: Option<String>,
        min_breadth: Option<i64>,
        bottleneck_tabaqah: Option<i64>,
        chain_count: Option<i64>,
        ilal_flags: Option<Vec<String>>,
    }

    #[derive(Debug, SurrealValue, serde::Serialize)]
    struct ChainRow {
        variant: Option<RecordId>,
        narrator_count: Option<i64>,
        has_chronology_conflict: Option<bool>,
        narrator_ids: Option<Vec<String>>,
    }

    #[derive(Debug, SurrealValue, serde::Serialize)]
    struct PivotRow {
        narrator: Option<RecordId>,
        bundle_coverage: Option<f64>,
        fan_out: Option<i64>,
        collector_diversity: Option<i64>,
        bypass_count: Option<i64>,
        is_bottleneck: Option<bool>,
    }

    let mut res = state
        .db
        .query(
            "SELECT * FROM isnad_analysis WHERE family = $fid LIMIT 1;\
             SELECT * FROM chain_assessment WHERE family = $fid;\
             SELECT * FROM narrator_pivot WHERE family = $fid ORDER BY bundle_coverage DESC;",
        )
        .bind(("fid", fid))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let isnad: Option<IsnadRow> = res.take(0).unwrap_or(None);
    let chains: Vec<ChainRow> = res.take(1).unwrap_or_default();
    let pivots: Vec<PivotRow> = res.take(2).unwrap_or_default();

    let chains_json: Vec<serde_json::Value> = chains
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "variant_id": c.variant.as_ref().map(record_id_key_string).unwrap_or_default(),
                "narrator_count": c.narrator_count,
                "has_chronology_conflict": c.has_chronology_conflict,
                "narrator_ids": c.narrator_ids,
            })
        })
        .collect();

    let pivots_json: Vec<serde_json::Value> = pivots
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "narrator_id": p.narrator.as_ref().map(record_id_key_string).unwrap_or_default(),
                "bundle_coverage": p.bundle_coverage,
                "fan_out": p.fan_out,
                "collector_diversity": p.collector_diversity,
                "bypass_count": p.bypass_count,
                "is_bottleneck": p.is_bottleneck,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "analysis": isnad,
        "chains": chains_json,
        "pivots": pivots_json,
    })))
}

pub async fn narrator_isnad_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    #[derive(Debug, SurrealValue, serde::Serialize)]
    struct PivotInfo {
        family: Option<RecordId>,
        is_bottleneck: Option<bool>,
    }
    let mut res = state
        .db
        .query("SELECT family, is_bottleneck FROM narrator_pivot WHERE narrator = $nid")
        .bind(("nid", rid("narrator", &id)))
        .await
        .unwrap();
    let rows: Vec<PivotInfo> = res.take(0).unwrap_or_default();

    let pivot_count = rows.len();
    let bottleneck_count = rows
        .iter()
        .filter(|r| r.is_bottleneck == Some(true))
        .count();
    let families: Vec<String> = rows
        .iter()
        .filter_map(|r| r.family.as_ref().map(record_id_key_string))
        .collect();

    Json(serde_json::json!({
        "narrator_id": id,
        "pivot_family_count": pivot_count,
        "bottleneck_family_count": bottleneck_count,
        "families": families,
    }))
}

pub async fn matn_diff_handler(
    State(state): State<AppState>,
    Query(params): Query<DiffParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let a_id = params.a.ok_or(StatusCode::BAD_REQUEST)?;
    let b_id = params.b.ok_or(StatusCode::BAD_REQUEST)?;

    // Fetch both hadiths in a single multi-statement query
    let mut res = state
        .db
        .query(format!(
            "SELECT {HADITH_FIELDS} FROM $rid_a; SELECT {HADITH_FIELDS} FROM $rid_b;"
        ))
        .bind(("rid_a", rid("hadith", &a_id)))
        .bind(("rid_b", rid("hadith", &b_id)))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hadith_a: Option<Hadith> = res.take(0).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hadith_a = hadith_a.ok_or(StatusCode::NOT_FOUND)?;
    let hadith_b: Option<Hadith> = res.take(1).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hadith_b = hadith_b.ok_or(StatusCode::NOT_FOUND)?;

    // Prefer matn (body without sanad) for diffing; fall back to full text
    let text_a = hadith_a
        .matn
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(hadith_a.text_ar.as_deref())
        .or(hadith_a.text_en.as_deref())
        .unwrap_or("");
    let text_b = hadith_b
        .matn
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(hadith_b.text_ar.as_deref())
        .or(hadith_b.text_en.as_deref())
        .unwrap_or("");

    let result = analysis::matn_diff::diff_matn(text_a, text_b, &a_id, &b_id);
    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct DiffParams {
    pub a: Option<String>,
    pub b: Option<String>,
}

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

#[derive(Deserialize)]
pub struct ExportParams {
    pub format: Option<String>,
}

// ── Helper result types ──

#[derive(Debug, SurrealValue)]
struct CountResult {
    c: i64,
}

#[derive(Debug, SurrealValue)]
pub struct NarratorWithCount {
    pub id: Option<RecordId>,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub search_name: Option<String>,
    pub generation: Option<String>,
    pub bio: Option<String>,
    pub kunya: Option<String>,
    pub death_year: Option<i64>,
    pub hadith_count: Option<i64>,
}

#[derive(Debug, SurrealValue)]
struct NarratorsResult {
    narrators: Vec<Narrator>,
}

#[derive(Debug, SurrealValue)]
struct HadithsResult {
    hadiths: Vec<Hadith>,
}

#[derive(Debug, SurrealValue)]
struct TeachersResult {
    teachers: Vec<Narrator>,
}

#[derive(Debug, SurrealValue)]
struct StudentsResult {
    students: Vec<Narrator>,
}

#[derive(Debug, SurrealValue)]
struct HeardFromEdge {
    in_id: RecordId,
    out_id: RecordId,
    chain_position: Option<i64>,
}

// ── Isnad Search endpoints ──

pub async fn narrator_autocomplete(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteParams>,
) -> impl IntoResponse {
    let q = params.q.trim().to_string();
    if q.is_empty() {
        return Json(Vec::<ApiNarratorWithCount>::new());
    }
    let limit = params.limit.unwrap_or(8);
    let slug = crate::quran::ingest::strip_arabic_diacritics(&q);

    let sql = "SELECT * FROM narrator \
        WHERE string::lowercase(name_en) CONTAINS string::lowercase($q) \
           OR name_ar CONTAINS $q \
           OR kunya CONTAINS $q \
           OR search_name CONTAINS $slug \
           OR $q INSIDE aliases \
        ORDER BY hadith_count DESC \
        LIMIT $limit";

    let narrators: Vec<NarratorWithCount> = match state
        .db
        .query(sql)
        .bind(("q", q))
        .bind(("slug", slug))
        .bind(("limit", limit))
        .await
    {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Autocomplete query failed: {e}");
            vec![]
        }
    };

    Json(
        narrators
            .into_iter()
            .map(|n| ApiNarratorWithCount {
                id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
                name_ar: n.name_ar,
                name_en: n.name_en,
                generation: n.generation,
                bio: n.bio,
                kunya: n.kunya,
                death_year: n.death_year,
                hadith_count: n.hadith_count.unwrap_or(0),
            })
            .collect::<Vec<_>>(),
    )
}

pub async fn isnad_search(
    State(state): State<AppState>,
    Json(body): Json<IsnadSearchRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let limit = body.limit.unwrap_or(20).min(100);
    let mode = body.mode.as_deref().unwrap_or("loose");

    if body.narrator_ids.len() < 2 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 1. Resolve narrator IDs and collect RecordIds
    let mut narrators: Vec<Narrator> = Vec::new();
    let mut narrator_rids: Vec<RecordId> = Vec::new();
    for slug in &body.narrator_ids {
        let nrid = rid("narrator", slug);
        let mut res = state
            .db
            .query("SELECT * FROM $rid")
            .bind(("rid", nrid))
            .await
            .map_err(|e| {
                tracing::error!("Narrator lookup failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        let n: Option<Narrator> = res.take(0).unwrap_or(None);
        let n = n.ok_or(StatusCode::NOT_FOUND)?;
        narrator_rids.push(n.id.clone().unwrap());
        narrators.push(n);
    }

    // 2. Two-step LET query: find hadiths where all narrators appear in the chain
    let narrator_count = narrator_rids.len() as i64;
    let sql = format!(
        "LET $matched = (SELECT VALUE out FROM (\
            SELECT out, count() AS c FROM narrates \
            WHERE in IN $narrator_ids \
            GROUP BY out\
        ) WHERE c = $narrator_count); \
        SELECT {HADITH_SEARCH_FIELDS} FROM hadith \
        WHERE id IN $matched LIMIT $limit"
    );

    let mut res = state
        .db
        .query(&sql)
        .bind(("narrator_ids", narrator_rids.clone()))
        .bind(("narrator_count", narrator_count))
        .bind(("limit", limit))
        .await
        .map_err(|e| {
            tracing::error!("Isnad search failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    // LET is statement 0, SELECT is statement 1
    let hadiths: Vec<HadithSearchResult> = res.take(1).unwrap_or_default();

    // 3. For strict mode: verify heard_from edges between consecutive narrator pairs
    let hadiths = if mode == "strict" && narrators.len() >= 2 {
        filter_strict_chains(&state.db, &narrators, hadiths)
            .await
            .map_err(|e| {
                tracing::error!("Strict chain filter failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        hadiths
    };

    let total = hadiths.len();
    let api_narrators: Vec<ApiNarratorSearchResult> = narrators
        .iter()
        .map(|n| ApiNarratorSearchResult {
            id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n.name_ar.clone(),
            name_en: n.name_en.clone(),
            generation: n.generation.clone(),
            hadith_count: n.hadith_count,
        })
        .collect();
    let api_hadiths: Vec<ApiHadithSearchResult> = hadiths
        .into_iter()
        .map(|h| ApiHadithSearchResult {
            id: h.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            hadith_number: h.hadith_number,
            collection_id: h.collection_id,
            text_ar: h.text_ar,
            text_en: h.text_en,
            narrator_text: h.narrator_text,
            score: None,
        })
        .collect();

    Ok(Json(IsnadSearchResponse {
        narrators: api_narrators,
        hadiths: api_hadiths,
        mode: mode.to_string(),
        total,
    }))
}

/// For strict mode: keep only hadiths where consecutive narrator pairs have heard_from edges.
async fn filter_strict_chains(
    db: &surrealdb::Surreal<crate::db::Db>,
    narrators: &[Narrator],
    hadiths: Vec<HadithSearchResult>,
) -> anyhow::Result<Vec<HadithSearchResult>> {
    #[derive(Debug, SurrealValue)]
    struct CountRow {
        c: i64,
    }

    let mut result = Vec::new();
    for h in hadiths {
        let hadith_rid = h.id.clone().unwrap();
        let mut valid = true;
        // Check consecutive pairs: narrators[0] heard from narrators[1], etc.
        for pair in narrators.windows(2) {
            let student = pair[0].id.as_ref().unwrap();
            let teacher = pair[1].id.as_ref().unwrap();
            let mut res = db
                .query(
                    "SELECT count() AS c FROM heard_from \
                     WHERE in = $student AND out = $teacher AND hadith_ref = $hid \
                     GROUP ALL",
                )
                .bind(("student", student.clone()))
                .bind(("teacher", teacher.clone()))
                .bind(("hid", hadith_rid.clone()))
                .await?;
            let row: Option<CountRow> = res.take(0).unwrap_or(None);
            if row.map(|r| r.c).unwrap_or(0) == 0 {
                valid = false;
                break;
            }
        }
        if valid {
            result.push(h);
        }
    }
    Ok(result)
}

pub async fn common_narrators(
    State(state): State<AppState>,
    Query(params): Query<CommonNarratorsParams>,
) -> Result<impl IntoResponse, StatusCode> {
    // 1. Resolve both narrators
    let nrid1 = rid("narrator", &params.a);
    let nrid2 = rid("narrator", &params.b);

    let mut res = state
        .db
        .query("SELECT * FROM $rid1; SELECT * FROM $rid2")
        .bind(("rid1", nrid1))
        .bind(("rid2", nrid2))
        .await
        .map_err(|e| {
            tracing::error!("Common narrators lookup failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let n1: Option<Narrator> = res.take(0).unwrap_or(None);
    let n2: Option<Narrator> = res.take(1).unwrap_or(None);
    let n1 = n1.ok_or(StatusCode::NOT_FOUND)?;
    let n2 = n2.ok_or(StatusCode::NOT_FOUND)?;

    let nid1 = n1.id.as_ref().unwrap().clone();
    let nid2 = n2.id.as_ref().unwrap().clone();

    // 2. Get hadith sets for each narrator
    #[derive(Debug, SurrealValue)]
    struct OutId {
        out: RecordId,
    }
    let mut res = state
        .db
        .query(
            "SELECT out FROM narrates WHERE in = $nid1; \
             SELECT out FROM narrates WHERE in = $nid2",
        )
        .bind(("nid1", nid1.clone()))
        .bind(("nid2", nid2.clone()))
        .await
        .map_err(|e| {
            tracing::error!("Common narrators hadith query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let set1: Vec<OutId> = res.take(0).unwrap_or_default();
    let set2: Vec<OutId> = res.take(1).unwrap_or_default();

    let ids2: HashSet<String> = set2.iter().map(|r| record_id_string(&r.out)).collect();
    let shared: Vec<RecordId> = set1
        .into_iter()
        .filter(|r| ids2.contains(&record_id_string(&r.out)))
        .map(|r| r.out)
        .collect();

    if shared.is_empty() {
        let api_n1 = ApiNarratorSearchResult {
            id: n1.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n1.name_ar.clone(),
            name_en: n1.name_en.clone(),
            generation: n1.generation.clone(),
            hadith_count: n1.hadith_count,
        };
        let api_n2 = ApiNarratorSearchResult {
            id: n2.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n2.name_ar.clone(),
            name_en: n2.name_en.clone(),
            generation: n2.generation.clone(),
            hadith_count: n2.hadith_count,
        };
        return Ok(Json(CommonNarratorsResponse {
            narrator1: api_n1,
            narrator2: api_n2,
            common: vec![],
        }));
    }

    // 3. Find narrators who also narrate the shared hadiths (excluding the two inputs)
    let mut res = state
        .db
        .query(
            "SELECT in AS narrator FROM narrates \
             WHERE out IN $shared AND in != $nid1 AND in != $nid2",
        )
        .bind(("shared", shared))
        .bind(("nid1", nid1))
        .bind(("nid2", nid2))
        .await
        .map_err(|e| {
            tracing::error!("Common narrators query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    #[derive(Debug, SurrealValue)]
    struct NarratorRef {
        narrator: RecordId,
    }
    let refs: Vec<NarratorRef> = res.take(0).unwrap_or_default();

    // Deduplicate
    let mut seen = HashSet::new();
    let unique_ids: Vec<RecordId> = refs
        .into_iter()
        .filter(|r| seen.insert(record_id_string(&r.narrator)))
        .map(|r| r.narrator)
        .collect();

    // Fetch narrator details
    let common: Vec<NarratorWithCount> = if unique_ids.is_empty() {
        vec![]
    } else {
        let mut res = state
            .db
            .query("SELECT * FROM narrator WHERE id IN $ids ORDER BY hadith_count DESC LIMIT 50")
            .bind(("ids", unique_ids))
            .await
            .map_err(|e| {
                tracing::error!("Common narrators detail query failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        res.take(0).unwrap_or_default()
    };

    let api_n1 = ApiNarratorSearchResult {
        id: n1.id.as_ref().map(record_id_key_string).unwrap_or_default(),
        name_ar: n1.name_ar.clone(),
        name_en: n1.name_en.clone(),
        generation: n1.generation.clone(),
        hadith_count: n1.hadith_count,
    };
    let api_n2 = ApiNarratorSearchResult {
        id: n2.id.as_ref().map(record_id_key_string).unwrap_or_default(),
        name_ar: n2.name_ar.clone(),
        name_en: n2.name_en.clone(),
        generation: n2.generation.clone(),
        hadith_count: n2.hadith_count,
    };
    let api_common: Vec<ApiNarratorWithCount> = common
        .into_iter()
        .map(|n| ApiNarratorWithCount {
            id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n.name_ar,
            name_en: n.name_en,
            generation: n.generation,
            bio: n.bio,
            kunya: n.kunya,
            death_year: n.death_year,
            hadith_count: n.hadith_count.unwrap_or(0),
        })
        .collect();

    Ok(Json(CommonNarratorsResponse {
        narrator1: api_n1,
        narrator2: api_n2,
        common: api_common,
    }))
}

// ── Unified Quran & Sunnah endpoints ──

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
        match crate::unified::search_unified_text_only(&state.db, &query, limit, page).await {
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

    match crate::unified::search_unified(
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

    let result = crate::agentic_rag::ask_agentic(
        llm.as_ref(),
        &state.db,
        embedder.as_ref(),
        &question,
        &opts,
        crate::agentic_rag::AskScope::Both,
    )
    .await
    .map_err(|e| {
        tracing::error!("Agentic RAG ask failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    use crate::agentic_rag::AgenticResult;
    use crate::quran::models::ApiAyahSearchResult;

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
