use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::models::NoteRef;
use crate::services::notes::{NoteListFilter, NotePatch, NotebookPatch};

use super::AppState;

/// `now_iso` lives in `services::notes` — re-export so existing callers (and
/// the OpenAPI spec) don't break.
pub use crate::services::notes::now_iso;

/// Map a service-side `anyhow::Error` to the right HTTP status using the
/// per-service marker functions.
fn status_for(e: &anyhow::Error) -> StatusCode {
    if crate::services::notes::is_not_found(e) {
        StatusCode::NOT_FOUND
    } else if crate::services::notes::is_bad_request(e) {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

// ── Query parameter types ──

#[derive(Deserialize)]
pub struct NoteListParams {
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub tag: Option<String>,
    pub color: Option<String>,
    pub q: Option<String>,
    pub notebook_id: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct BulkRefsParams {
    pub ref_type: String,
    pub ref_ids: String, // comma-separated
}

#[derive(Deserialize)]
pub struct CreateNoteRequest {
    pub ref_type: String,
    pub ref_id: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub color: Option<String>,
    pub tags: Option<Vec<String>>,
    pub refs: Option<Vec<NoteRef>>,
    pub notebook_id: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateNoteRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub color: Option<String>,
    pub tags: Option<Vec<String>>,
    pub refs: Option<Vec<NoteRef>>,
    pub notebook_id: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateNoteRefsRequest {
    pub action: String, // "add" | "remove"
    #[serde(rename = "ref")]
    pub note_ref: NoteRef,
}

#[derive(Deserialize)]
pub struct UpdateRefAnnotationRequest {
    pub annotation: String,
}

// ── Handlers ──

pub async fn list_notes(
    State(state): State<AppState>,
    Query(params): Query<NoteListParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let filter = NoteListFilter {
        ref_type: params.ref_type,
        ref_id: params.ref_id,
        color: params.color,
        q: params.q,
        tag: params.tag,
        notebook_id: params.notebook_id,
    };
    crate::services::notes::list(&state, &filter, page, limit)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("note list failed: {e}");
            status_for(&e)
        })
}

pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::get(&state, &id)
        .await
        .map(Json)
        .map_err(|e| status_for(&e))
}

pub async fn create_note(
    State(state): State<AppState>,
    Json(body): Json<CreateNoteRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::create(
        &state,
        body.ref_type,
        body.ref_id,
        body.title,
        body.content,
        body.color,
        body.tags,
        body.refs,
        body.notebook_id,
    )
    .await
    .map(Json)
    .map_err(|e| {
        tracing::error!("create note failed: {e}");
        status_for(&e)
    })
}

pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNoteRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let patch = NotePatch {
        title: body.title,
        content: body.content,
        color: body.color,
        tags: body.tags,
        refs: body.refs,
        notebook_id: body.notebook_id,
    };
    crate::services::notes::update(&state, &id, patch)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("update note failed: {e}");
            status_for(&e)
        })
}

pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::delete(&state, &id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            tracing::error!("delete note failed: {e}");
            status_for(&e)
        })
}

pub async fn bulk_note_refs(
    State(state): State<AppState>,
    Query(params): Query<BulkRefsParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let ref_ids: Vec<String> = params
        .ref_ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    crate::services::notes::bulk_refs(&state, &params.ref_type, &ref_ids)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("bulk note refs failed: {e}");
            status_for(&e)
        })
}

pub async fn list_tags(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::list_tags(&state)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("list tags failed: {e}");
            status_for(&e)
        })
}

pub async fn export_notes(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::export(&state)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("export notes failed: {e}");
            status_for(&e)
        })
}

pub async fn update_note_refs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNoteRefsRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::update_note_refs(&state, &id, &body.action, body.note_ref)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("update note refs failed: {e}");
            status_for(&e)
        })
}

pub async fn update_ref_annotation(
    State(state): State<AppState>,
    Path((id, idx)): Path<(String, usize)>,
    Json(body): Json<UpdateRefAnnotationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::update_ref_annotation(&state, &id, idx, body.annotation)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("update ref annotation failed: {e}");
            status_for(&e)
        })
}

// ── Notebook Handlers ──

#[derive(Deserialize)]
pub struct CreateNotebookRequest {
    pub name: String,
    pub emoji: Option<String>,
    pub parent_id: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateNotebookRequest {
    pub name: Option<String>,
    pub emoji: Option<String>,
    pub parent_id: Option<String>,
    pub sort_order: Option<i32>,
}

pub async fn list_notebooks(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::list_notebooks(&state)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("list notebooks failed: {e}");
            status_for(&e)
        })
}

pub async fn create_notebook(
    State(state): State<AppState>,
    Json(body): Json<CreateNotebookRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::create_notebook(&state, body.name, body.emoji, body.parent_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("create notebook failed: {e}");
            status_for(&e)
        })
}

pub async fn update_notebook(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNotebookRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let patch = NotebookPatch {
        name: body.name,
        emoji: body.emoji,
        parent_id: body.parent_id,
        sort_order: body.sort_order,
    };
    crate::services::notes::update_notebook(&state, &id, patch)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("update notebook failed: {e}");
            status_for(&e)
        })
}

pub async fn delete_notebook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    crate::services::notes::delete_notebook(&state, &id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            tracing::error!("delete notebook failed: {e}");
            status_for(&e)
        })
}
