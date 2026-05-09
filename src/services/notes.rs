//! Notes & notebooks services.
//!
//! Notes live in a single global namespace — no per-user scoping in v1. A
//! later session-id story can re-introduce filtering by threading an owner
//! id through these service signatures.
//!
//! Mutating MCP tools must be gated by `--enable-mutations` at the MCP
//! boundary; the service layer doesn't gate.

use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use surrealdb::types::SurrealValue;

use crate::models::{
    ApiNotebook, ApiUserNote, NoteRef, Notebook, PaginatedResponse, UserNote, make_record_id,
};
use crate::web::AppState;

// ── Helpers ────────────────────────────────────────────────────────────────

/// Generate ISO 8601 timestamp without depending on `chrono`.
pub fn now_iso() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let s = time_secs % 60;
    let mut y = 1970i64;
    let mut remaining_days = days as i64;
    loop {
        let dy = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining_days < dy {
            break;
        }
        remaining_days -= dy;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    for md in month_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        m += 1;
    }
    format!(
        "{y:04}-{:02}-{:02}T{hours:02}:{mins:02}:{s:02}Z",
        m + 1,
        remaining_days + 1
    )
}

/// "note not found" / "notebook not found" — flagged 404 by the HTTP layer.
pub fn is_not_found(e: &anyhow::Error) -> bool {
    let msg = e.to_string();
    msg.starts_with("note not found:") || msg.starts_with("notebook not found:")
}

pub fn is_bad_request(e: &anyhow::Error) -> bool {
    e.to_string().starts_with("note bad_request:")
}

// ── Filter / patch types (used by both HTTP and MCP) ───────────────────────

/// Optional filters for `list` — every field is independent.
#[derive(Debug, Default, Clone)]
pub struct NoteListFilter {
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub color: Option<String>,
    pub q: Option<String>,
    pub tag: Option<String>,
    /// Pass `Some("__uncategorized__")` to filter notes with `notebook_id IS NONE`.
    pub notebook_id: Option<String>,
}

/// Patch payload for `update` — every field is optional, at least one must be set.
#[derive(Debug, Default, Clone)]
pub struct NotePatch {
    pub title: Option<String>,
    pub content: Option<String>,
    pub color: Option<String>,
    pub tags: Option<Vec<String>>,
    pub refs: Option<Vec<NoteRef>>,
    pub notebook_id: Option<String>,
}

/// Patch payload for `update_notebook`.
#[derive(Debug, Default, Clone)]
pub struct NotebookPatch {
    pub name: Option<String>,
    pub emoji: Option<String>,
    pub parent_id: Option<String>,
    pub sort_order: Option<i32>,
}

// ── Note CRUD ──────────────────────────────────────────────────────────────

/// Paginated note list with optional filters. Sorted by `updated_at DESC`.
pub async fn list(
    state: &AppState,
    filter: &NoteListFilter,
    page: usize,
    limit: usize,
) -> Result<PaginatedResponse<ApiUserNote>> {
    let page = page.max(1);
    let limit = limit.clamp(1, 200);
    let offset = (page - 1) * limit;

    let mut conditions: Vec<String> = Vec::new();
    if filter.ref_type.is_some() {
        conditions.push("ref_type = $ref_type".to_string());
    }
    if filter.ref_id.is_some() {
        conditions.push("ref_id = $ref_id".to_string());
    }
    if filter.color.is_some() {
        conditions.push("color = $color".to_string());
    }
    if filter.q.is_some() {
        conditions.push(
            "(string::lowercase(content) CONTAINS string::lowercase($q) \
              OR string::lowercase(title) CONTAINS string::lowercase($q))"
                .to_string(),
        );
    }
    if filter.tag.is_some() {
        conditions.push("$tag IN tags".to_string());
    }
    if let Some(nb) = &filter.notebook_id {
        if nb == "__uncategorized__" {
            conditions.push("notebook_id IS NONE".to_string());
        } else {
            conditions.push("notebook_id = $notebook_id".to_string());
        }
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };
    let sql = format!(
        "SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at \
         FROM user_note {where_clause} ORDER BY updated_at DESC LIMIT {limit} START {offset}"
    );

    let mut q = state.db.query(&sql);
    if let Some(rt) = &filter.ref_type {
        q = q.bind(("ref_type", rt.clone()));
    }
    if let Some(ri) = &filter.ref_id {
        q = q.bind(("ref_id", ri.clone()));
    }
    if let Some(c) = &filter.color {
        q = q.bind(("color", c.clone()));
    }
    if let Some(qs) = &filter.q {
        q = q.bind(("q", qs.clone()));
    }
    if let Some(tag) = &filter.tag {
        q = q.bind(("tag", tag.clone()));
    }
    if let Some(nb) = &filter.notebook_id {
        if nb != "__uncategorized__" {
            q = q.bind(("notebook_id", nb.clone()));
        }
    }

    let notes: Vec<UserNote> = q
        .await
        .context("note list query failed")?
        .take(0)
        .unwrap_or_default();
    let has_more = notes.len() == limit;
    Ok(PaginatedResponse {
        data: notes.into_iter().map(ApiUserNote::from).collect(),
        page,
        limit,
        has_more,
        total: None,
    })
}

/// Single note (404 if missing).
pub async fn get(state: &AppState, id: &str) -> Result<ApiUserNote> {
    let mut res = state
        .db
        .query(
            "SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at \
             FROM $rid",
        )
        .bind(("rid", make_record_id("user_note", id)))
        .await
        .context("note lookup failed")?;
    let note: UserNote = res
        .take::<Option<UserNote>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("note not found: {id}"))?;
    Ok(ApiUserNote::from(note))
}

/// Create a new note.
#[allow(clippy::too_many_arguments)]
pub async fn create(
    state: &AppState,
    ref_type: String,
    ref_id: Option<String>,
    title: Option<String>,
    content: Option<String>,
    color: Option<String>,
    tags: Option<Vec<String>>,
    refs: Option<Vec<NoteRef>>,
    notebook_id: Option<String>,
) -> Result<ApiUserNote> {
    let content = content.unwrap_or_default();
    let color = color.unwrap_or_else(|| "yellow".to_string());
    let refs_json = refs.map(|r| serde_json::to_string(&r).unwrap_or_else(|_| "[]".to_string()));
    let now = now_iso();

    let mut res = state
        .db
        .query(
            "CREATE user_note CONTENT { ref_type: $ref_type, ref_id: $ref_id, \
             title: $title, content: $content, color: $color, tags: $tags, refs: $refs, \
             notebook_id: $notebook_id, created_at: $now, updated_at: $now }",
        )
        .bind(("ref_type", ref_type))
        .bind(("ref_id", ref_id))
        .bind(("title", title))
        .bind(("content", content))
        .bind(("color", color))
        .bind(("tags", tags))
        .bind(("refs", refs_json))
        .bind(("notebook_id", notebook_id))
        .bind(("now", now))
        .await
        .context("create note failed")?;
    let note: UserNote = res
        .take::<Option<UserNote>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("create note returned no row"))?;
    Ok(ApiUserNote::from(note))
}

/// Patch a note. Returns `bad_request` when the patch is empty.
pub async fn update(state: &AppState, id: &str, patch: NotePatch) -> Result<ApiUserNote> {
    let mut update = serde_json::Map::new();
    if let Some(t) = patch.title {
        update.insert("title".to_string(), Value::String(t));
    }
    if let Some(c) = patch.content {
        update.insert("content".to_string(), Value::String(c));
    }
    if let Some(c) = patch.color {
        update.insert("color".to_string(), Value::String(c));
    }
    if let Some(t) = patch.tags {
        update.insert("tags".to_string(), serde_json::json!(t));
    }
    if let Some(r) = patch.refs {
        let refs_json = serde_json::to_string(&r).unwrap_or_else(|_| "[]".to_string());
        update.insert("refs".to_string(), Value::String(refs_json));
    }
    if let Some(n) = patch.notebook_id {
        update.insert("notebook_id".to_string(), Value::String(n));
    }
    if update.is_empty() {
        return Err(anyhow!(
            "note bad_request: patch must include at least one field"
        ));
    }
    update.remove("created_at");
    update.insert("updated_at".to_string(), Value::String(now_iso()));

    let mut res = state
        .db
        .query("UPDATE $rid MERGE $data RETURN AFTER")
        .bind(("rid", make_record_id("user_note", id)))
        .bind(("data", Value::Object(update)))
        .await
        .context("update note failed")?;
    let note: UserNote = res
        .take::<Option<UserNote>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("note not found: {id}"))?;
    Ok(ApiUserNote::from(note))
}

/// Delete a note.
pub async fn delete(state: &AppState, id: &str) -> Result<()> {
    state
        .db
        .query("DELETE $rid")
        .bind(("rid", make_record_id("user_note", id)))
        .await
        .context("delete note failed")?;
    Ok(())
}

/// For a list of `(ref_type, ref_id)` pairs, return per-ref note count + color.
/// Map key is the ref_id; value is `{ color, count }`.
pub async fn bulk_refs(state: &AppState, ref_type: &str, ref_ids: &[String]) -> Result<Value> {
    if ref_ids.is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    #[derive(Debug, SurrealValue)]
    struct RefCount {
        ref_id: Option<String>,
        color: String,
        count: i64,
    }
    let rows: Vec<RefCount> = state
        .db
        .query(
            "SELECT ref_id, color, count() AS count FROM user_note \
             WHERE ref_type = $ref_type AND ref_id IN $ref_ids GROUP BY ref_id",
        )
        .bind(("ref_type", ref_type.to_string()))
        .bind(("ref_ids", ref_ids.to_vec()))
        .await
        .context("bulk note refs query failed")?
        .take(0)
        .unwrap_or_default();

    let mut result = serde_json::Map::new();
    for rc in rows {
        if let Some(rid) = rc.ref_id {
            result.insert(
                rid,
                serde_json::json!({ "color": rc.color, "count": rc.count }),
            );
        }
    }
    Ok(Value::Object(result))
}

/// Distinct sorted list of all tags in use.
pub async fn list_tags(state: &AppState) -> Result<Vec<String>> {
    #[derive(Debug, SurrealValue)]
    struct TagRow {
        tags: Option<Vec<String>>,
    }
    let rows: Vec<TagRow> = state
        .db
        .query("SELECT tags FROM user_note WHERE tags IS NOT NONE")
        .await
        .context("list tags query failed")?
        .take(0)
        .unwrap_or_default();
    let mut all_tags: Vec<String> = rows
        .into_iter()
        .flat_map(|r| r.tags.unwrap_or_default())
        .collect();
    all_tags.sort();
    all_tags.dedup();
    Ok(all_tags)
}

/// Export every note, ordered by `updated_at DESC`.
pub async fn export(state: &AppState) -> Result<Vec<ApiUserNote>> {
    let notes: Vec<UserNote> = state
        .db
        .query(
            "SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at \
             FROM user_note ORDER BY updated_at DESC",
        )
        .await
        .context("export notes query failed")?
        .take(0)
        .unwrap_or_default();
    Ok(notes.into_iter().map(ApiUserNote::from).collect())
}

/// Add or remove a single ref from a note. `action` must be `"add"` or `"remove"`.
pub async fn update_note_refs(
    state: &AppState,
    id: &str,
    action: &str,
    note_ref: NoteRef,
) -> Result<ApiUserNote> {
    if action != "add" && action != "remove" {
        return Err(anyhow!(
            "note bad_request: action must be 'add' or 'remove'"
        ));
    }
    let mut res = state
        .db
        .query(
            "SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at \
             FROM $rid",
        )
        .bind(("rid", make_record_id("user_note", id)))
        .await
        .context("note lookup failed")?;
    let note: UserNote = res
        .take::<Option<UserNote>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("note not found: {id}"))?;
    let mut refs: Vec<NoteRef> = note
        .refs
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    match action {
        "add" => {
            let exists = refs
                .iter()
                .any(|r| r.ref_type == note_ref.ref_type && r.ref_id == note_ref.ref_id);
            if !exists {
                refs.push(note_ref);
            }
        }
        "remove" => {
            refs.retain(|r| !(r.ref_type == note_ref.ref_type && r.ref_id == note_ref.ref_id))
        }
        _ => unreachable!(),
    }

    let refs_json = serde_json::to_string(&refs).unwrap_or_else(|_| "[]".to_string());
    let mut res = state
        .db
        .query("UPDATE $rid SET refs = $refs, updated_at = $updated_at RETURN AFTER")
        .bind(("rid", make_record_id("user_note", id)))
        .bind(("refs", refs_json))
        .bind(("updated_at", now_iso()))
        .await
        .context("update note refs failed")?;
    let note: UserNote = res
        .take::<Option<UserNote>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("note not found: {id}"))?;
    Ok(ApiUserNote::from(note))
}

/// Replace the annotation on an existing ref by index.
pub async fn update_ref_annotation(
    state: &AppState,
    id: &str,
    idx: usize,
    annotation: String,
) -> Result<ApiUserNote> {
    let mut res = state
        .db
        .query(
            "SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at \
             FROM $rid",
        )
        .bind(("rid", make_record_id("user_note", id)))
        .await
        .context("note lookup failed")?;
    let note: UserNote = res
        .take::<Option<UserNote>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("note not found: {id}"))?;
    let mut refs: Vec<NoteRef> = note
        .refs
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if idx >= refs.len() {
        return Err(anyhow!(
            "note bad_request: ref index {idx} out of range (refs len = {})",
            refs.len()
        ));
    }
    refs[idx].annotation = Some(annotation);
    let refs_json = serde_json::to_string(&refs).unwrap_or_else(|_| "[]".to_string());

    let mut res = state
        .db
        .query("UPDATE $rid SET refs = $refs, updated_at = $updated_at RETURN AFTER")
        .bind(("rid", make_record_id("user_note", id)))
        .bind(("refs", refs_json))
        .bind(("updated_at", now_iso()))
        .await
        .context("update ref annotation failed")?;
    let note: UserNote = res
        .take::<Option<UserNote>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("note not found: {id}"))?;
    Ok(ApiUserNote::from(note))
}

// ── Notebook CRUD ──────────────────────────────────────────────────────────

/// All notebooks, ordered by sort_order then created_at.
pub async fn list_notebooks(state: &AppState) -> Result<Vec<ApiNotebook>> {
    let notebooks: Vec<Notebook> = state
        .db
        .query(
            "SELECT *, <string>created_at AS created_at FROM notebook \
             ORDER BY sort_order ASC, created_at ASC",
        )
        .await
        .context("list notebooks failed")?
        .take(0)
        .unwrap_or_default();
    Ok(notebooks.into_iter().map(ApiNotebook::from).collect())
}

/// Create a new notebook.
pub async fn create_notebook(
    state: &AppState,
    name: String,
    emoji: Option<String>,
    parent_id: Option<String>,
) -> Result<ApiNotebook> {
    let now = now_iso();
    let mut res = state
        .db
        .query(
            "CREATE notebook CONTENT { name: $name, emoji: $emoji, \
             parent_id: $parent_id, sort_order: 0, created_at: $now }",
        )
        .bind(("name", name))
        .bind(("emoji", emoji))
        .bind(("parent_id", parent_id))
        .bind(("now", now))
        .await
        .context("create notebook failed")?;
    let nb: Notebook = res
        .take::<Option<Notebook>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("create notebook returned no row"))?;
    Ok(ApiNotebook::from(nb))
}

/// Patch a notebook. Self-parenting (`parent_id == id`) is rejected as bad_request.
pub async fn update_notebook(
    state: &AppState,
    id: &str,
    patch: NotebookPatch,
) -> Result<ApiNotebook> {
    let mut update = serde_json::Map::new();
    if let Some(name) = patch.name {
        update.insert("name".to_string(), Value::String(name));
    }
    if let Some(emoji) = patch.emoji {
        update.insert("emoji".to_string(), Value::String(emoji));
    }
    if let Some(pid) = patch.parent_id {
        if pid == id {
            return Err(anyhow!(
                "note bad_request: notebook cannot be its own parent"
            ));
        }
        update.insert("parent_id".to_string(), Value::String(pid));
    }
    if let Some(s) = patch.sort_order {
        update.insert("sort_order".to_string(), serde_json::json!(s));
    }
    if update.is_empty() {
        return Err(anyhow!(
            "note bad_request: notebook patch must include at least one field"
        ));
    }
    let mut res = state
        .db
        .query("UPDATE $rid MERGE $data RETURN AFTER")
        .bind(("rid", make_record_id("notebook", id)))
        .bind(("data", Value::Object(update)))
        .await
        .context("update notebook failed")?;
    let nb: Notebook = res
        .take::<Option<Notebook>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("notebook not found: {id}"))?;
    Ok(ApiNotebook::from(nb))
}

/// Delete a notebook + clear references from notes and child notebooks.
pub async fn delete_notebook(state: &AppState, id: &str) -> Result<()> {
    state
        .db
        .query("UPDATE user_note SET notebook_id = NONE WHERE notebook_id = $nb_id")
        .bind(("nb_id", id.to_string()))
        .await
        .context("clear notebook refs failed")?;
    state
        .db
        .query("UPDATE notebook SET parent_id = NONE WHERE parent_id = $nb_id")
        .bind(("nb_id", id.to_string()))
        .await
        .context("clear child notebook refs failed")?;
    state
        .db
        .query("DELETE $rid")
        .bind(("rid", make_record_id("notebook", id)))
        .await
        .context("delete notebook failed")?;
    Ok(())
}
