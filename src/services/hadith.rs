//! Hadith-domain services.
//!
//! Single source of truth for hadith retrieval / listing / detail composition.
//! Both `crate::web::handlers` and `crate::mcp::tools` call into here.

use anyhow::{Context, Result, anyhow};
use surrealdb::types::SurrealValue;

use crate::models::{
    ApiCollection, ApiHadith, ApiHadithDetail, ApiHadithSearchResult, ApiNarrator,
    ApiNarratorSearchResult, Collection, HADITH_FIELDS, HADITH_SEARCH_FIELDS, Hadith,
    HadithSearchResult, IsnadSearchResponse, Narrator, PaginatedResponse, make_record_id,
};
use crate::web::AppState;

/// Result of "is this id present" — distinguishes "not found" from a real
/// error so HTTP can map it to 404 and MCP can map it to `invalid_request`.
pub enum NotFound {}

/// List all hadith collections (Bukhari, Muslim, Abu Dawud, Tirmidhi, Nasai,
/// Ibn Majah) in `collection_id` order.
pub async fn list_collections(state: &AppState) -> Result<Vec<ApiCollection>> {
    let mut res = state
        .db
        .query("SELECT * FROM collection ORDER BY collection_id ASC")
        .await
        .context("collection list query failed")?;
    let books: Vec<Collection> = res.take(0).unwrap_or_default();
    Ok(books.into_iter().map(ApiCollection::from).collect())
}

/// Filters accepted by the paginated hadith list query.
///
/// Fields are independent — supplying multiple narrows the result set with AND
/// semantics. `book` (single) and `books` (multi) are unioned: a hadith
/// matches if its `collection_id` equals `book` OR is in `books`.
#[derive(Default)]
pub struct ListFilters {
    pub book: Option<i64>,
    pub books: Vec<i64>,
    pub number: Option<i64>,
    pub n_min: Option<i64>,
    pub n_max: Option<i64>,
    /// Narrator slugs (without the `narrator:` table prefix). A hadith matches
    /// when at least one of these narrators sits on its transmission chain.
    pub narrators: Vec<String>,
    /// Substring filter applied (case-insensitively) to `text_ar`, `text_en`,
    /// and `narrator_text`.
    pub q: Option<String>,
    /// `number_asc` (default) or `number_desc`.
    pub sort: Option<String>,
}

/// Paginated hadith list with optional filters. Returns `data` ordered by
/// `hadith_number` (asc by default, desc when `sort=number_desc`).
///
/// Pagination is offset-based and detects `has_more` via the size of the
/// returned page (no `total` count to keep the query cheap).
pub async fn list(
    state: &AppState,
    filters: ListFilters,
    page: usize,
    limit: usize,
) -> Result<PaginatedResponse<ApiHadith>> {
    let page = page.max(1);
    let limit = limit.clamp(1, 100);
    let offset = (page - 1) * limit;

    let ListFilters {
        book,
        books,
        number,
        n_min,
        n_max,
        narrators,
        q,
        sort,
    } = filters;

    let mut conditions: Vec<String> = Vec::new();

    // Collection filter — `book` (single) and `books` (multi) union into a
    // single `collection_id IN [...]` condition so callers can supply either.
    let mut book_ids: Vec<i64> = books;
    if let Some(b) = book {
        if !book_ids.contains(&b) {
            book_ids.push(b);
        }
    }
    if !book_ids.is_empty() {
        let ids = book_ids
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        conditions.push(format!("collection_id IN [{ids}]"));
    }

    if let Some(num) = number {
        conditions.push(format!("hadith_number = {num}"));
    }
    if let Some(min) = n_min {
        conditions.push(format!("hadith_number >= {min}"));
    }
    if let Some(max) = n_max {
        conditions.push(format!("hadith_number <= {max}"));
    }

    if q.is_some() {
        // Lowercase both sides so the substring match is case-insensitive
        // (matches the `narrator` autocomplete pattern in `services::search`).
        // `?? ''` guards against null fields, which would otherwise propagate
        // a NONE through the comparison.
        conditions.push(
            "(string::lowercase(text_ar ?? '') CONTAINS string::lowercase($q) \
             OR string::lowercase(text_en ?? '') CONTAINS string::lowercase($q) \
             OR string::lowercase(narrator_text ?? '') CONTAINS string::lowercase($q))"
                .to_string(),
        );
    }

    if !narrators.is_empty() {
        // Hadiths reachable via any of the supplied narrators' `narrates`
        // edges. Resolved as record ids so the `IN $narrator_ids` filter is
        // typesafe (mirrors `isnad_search` in this same file).
        conditions.push(
            "id IN (SELECT VALUE out FROM narrates WHERE in IN $narrator_ids)".to_string(),
        );
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let order = if sort.as_deref() == Some("number_desc") {
        "hadith_number DESC"
    } else {
        "hadith_number ASC"
    };

    let query = format!(
        "SELECT {HADITH_FIELDS} FROM hadith {where_clause} \
         ORDER BY {order} LIMIT {limit} START {offset}"
    );

    let mut q_builder = state.db.query(&query);
    if let Some(qs) = q {
        q_builder = q_builder.bind(("q", qs));
    }
    if !narrators.is_empty() {
        let narrator_rids: Vec<surrealdb::types::RecordId> = narrators
            .iter()
            .map(|s| make_record_id("narrator", s))
            .collect();
        q_builder = q_builder.bind(("narrator_ids", narrator_rids));
    }

    let mut res = q_builder.await.context("hadith list query failed")?;
    let hadiths: Vec<Hadith> = res.take(0).unwrap_or_default();
    let has_more = hadiths.len() == limit;

    Ok(PaginatedResponse {
        data: hadiths.into_iter().map(ApiHadith::from).collect(),
        page,
        limit,
        has_more,
        total: None,
    })
}

/// Hadith detail: hadith record + transmission chain (`narrates` edges) +
/// linked Quran ayahs (`references_hadith` edges) + semantically-similar
/// hadiths (`similar_to` edges). Returns `Err` with a not-found marker when
/// the id doesn't resolve.
pub async fn get_detail(state: &AppState, id: &str) -> Result<ApiHadithDetail> {
    let hrid = make_record_id("hadith", id);

    // Single multi-statement query to avoid 5 sequential round trips.
    let mut res = state
        .db
        .query(format!(
            "SELECT {HADITH_FIELDS} FROM $rid; \
             SELECT <-narrates<-narrator.* AS narrators FROM $rid; \
             SELECT in.id AS id, in.surah_number AS surah_number, in.ayah_number AS ayah_number, \
               in.text_ar AS text_ar, in.text_en AS text_en, in.tafsir_en AS tafsir_en \
               FROM references_hadith WHERE out = $rid ORDER BY surah_number, ayah_number; \
             SELECT ->similar_to->hadith.{{{HADITH_FIELDS}}} AS hadiths FROM $rid; \
             SELECT parallel_hadith_id.{{{HADITH_FIELDS}}} AS hadith FROM hadith_parallel \
               WHERE hadith_id = $rid ORDER BY score DESC;"
        ))
        .bind(("rid", hrid))
        .await
        .context("hadith detail query failed")?;

    let hadith: Hadith = res
        .take::<Option<Hadith>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("hadith not found: {id}"))?;

    #[derive(Debug, SurrealValue)]
    struct NarratorsResult {
        narrators: Vec<Narrator>,
    }
    let narrators: Vec<Narrator> = res
        .take::<Option<NarratorsResult>>(1)
        .unwrap_or(None)
        .map(|r| r.narrators)
        .unwrap_or_default();

    let linked_ayahs: Vec<crate::quran::models::ApiAyah> = {
        let ayahs: Vec<crate::quran::models::Ayah> = res.take(2).unwrap_or_default();
        ayahs
            .into_iter()
            .map(crate::quran::models::ApiAyah::from)
            .collect()
    };

    #[derive(Debug, SurrealValue)]
    struct HadithsResult {
        hadiths: Vec<Hadith>,
    }
    let similar_hadiths: Vec<ApiHadith> = res
        .take::<Option<HadithsResult>>(3)
        .unwrap_or(None)
        .map(|r| r.hadiths.into_iter().map(ApiHadith::from).collect())
        .unwrap_or_default();

    #[derive(Debug, SurrealValue)]
    struct ParallelRow {
        hadith: Hadith,
    }
    let parallel_rows: Vec<ParallelRow> = res.take(4).unwrap_or_default();
    let parallel_hadiths: Vec<ApiHadith> = parallel_rows
        .into_iter()
        .map(|r| ApiHadith::from(r.hadith))
        .collect();

    Ok(ApiHadithDetail {
        hadith: ApiHadith::from(hadith),
        narrators: narrators.into_iter().map(ApiNarrator::from).collect(),
        linked_ayahs,
        similar_hadiths,
        parallel_hadiths,
    })
}

/// Returns true when an `anyhow::Error` was produced by `get_detail` for a
/// missing id (i.e. message starts with "hadith not found:" or
/// "narrator not found:" — the latter from `isnad_search` resolving slugs).
/// Used by the HTTP and MCP layers to distinguish 404 from 500.
pub fn is_not_found(e: &anyhow::Error) -> bool {
    let msg = e.to_string();
    msg.starts_with("hadith not found:")
        || msg.starts_with("hadith id not found:")
        || msg.starts_with("narrator not found:")
}

/// Returns true when an `isnad_search` was called with bad input (e.g. fewer
/// than 2 narrators). Mapped to 400 / `invalid_request`.
pub fn is_bad_request(e: &anyhow::Error) -> bool {
    e.to_string().starts_with("isnad_search bad_request:")
}

/// Cytoscape-shaped isnad graph for one hadith — nodes for each narrator in
/// the chain, edges marked with `chain_position`.
pub async fn get_chain_graph(
    state: &AppState,
    hadith_id: &str,
) -> Result<crate::models::GraphData> {
    use crate::models::{
        GraphData, GraphEdge, GraphEdgeData, GraphNode, GraphNodeData, record_id_string,
    };
    use surrealdb::types::RecordId;

    let hrid = make_record_id("hadith", hadith_id);

    let narrators = match state
        .db
        .query("SELECT <-narrates<-narrator.* AS narrators FROM $rid")
        .bind(("rid", hrid.clone()))
        .await
    {
        Ok(mut r) => {
            #[derive(Debug, SurrealValue)]
            struct NarratorsRow {
                narrators: Vec<Narrator>,
            }
            r.take::<Option<NarratorsRow>>(0)
                .unwrap_or(None)
                .map(|r| r.narrators)
                .unwrap_or_default()
        }
        Err(e) => return Err(anyhow::Error::from(e).context("chain narrators query failed")),
    };

    #[derive(Debug, SurrealValue)]
    struct EdgeRow {
        in_id: RecordId,
        out_id: RecordId,
        chain_position: Option<i64>,
    }
    let edges: Vec<EdgeRow> = state
        .db
        .query(
            "SELECT in AS in_id, out AS out_id, chain_position \
             FROM heard_from WHERE hadith_ref = $rid ORDER BY chain_position",
        )
        .bind(("rid", hrid))
        .await
        .context("chain edges query failed")?
        .take(0)
        .unwrap_or_default();

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
    Ok(graph)
}

/// Find hadiths whose isnad chain contains the given narrator slugs.
///
/// `mode = "loose"` (default): narrators may appear in any order. `mode =
/// "strict"`: narrators must form a contiguous student → teacher sub-chain
/// in the order provided.
///
/// Errors: returns `narrator not found:<slug>` (mapped to 404 / invalid_request)
/// when any input slug doesn't resolve. Returns `bad_request` when fewer than
/// 2 narrator ids supplied.
pub async fn isnad_search(
    state: &crate::web::AppState,
    narrator_ids: &[String],
    mode: &str,
    limit: usize,
) -> Result<IsnadSearchResponse> {
    use surrealdb::types::RecordId;

    if narrator_ids.len() < 2 {
        return Err(anyhow!(
            "isnad_search bad_request: need at least 2 narrator ids"
        ));
    }
    let mode = if mode == "strict" { "strict" } else { "loose" };
    let limit = limit.clamp(1, 100);

    // 1. Resolve every input slug to a Narrator + RecordId.
    let mut narrators: Vec<Narrator> = Vec::with_capacity(narrator_ids.len());
    let mut narrator_rids: Vec<RecordId> = Vec::with_capacity(narrator_ids.len());
    for slug in narrator_ids {
        let nrid = make_record_id("narrator", slug);
        let n: Narrator = state
            .db
            .query("SELECT * FROM $rid")
            .bind(("rid", nrid))
            .await
            .context("narrator lookup failed")?
            .take::<Option<Narrator>>(0)
            .unwrap_or(None)
            .ok_or_else(|| anyhow!("narrator not found: {slug}"))?;
        narrator_rids.push(n.id.clone().unwrap());
        narrators.push(n);
    }

    // 2. Two-step LET query: find hadiths whose narrate-edges include EVERY input narrator.
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
        .bind(("narrator_ids", narrator_rids))
        .bind(("narrator_count", narrator_count))
        .bind(("limit", limit))
        .await
        .context("isnad search query failed")?;
    // LET is statement 0; SELECT is statement 1.
    let hadiths: Vec<HadithSearchResult> = res.take(1).unwrap_or_default();

    // 3. Strict mode: post-filter so consecutive (student, teacher) pairs have a heard_from edge.
    let hadiths = if mode == "strict" && narrators.len() >= 2 {
        filter_strict_chains(state, &narrators, hadiths).await?
    } else {
        hadiths
    };

    let total = hadiths.len();
    let api_narrators = narrators
        .iter()
        .map(|n| ApiNarratorSearchResult {
            id: n
                .id
                .as_ref()
                .map(crate::models::record_id_key_string)
                .unwrap_or_default(),
            name_ar: n.name_ar.clone(),
            name_en: n.name_en.clone(),
            generation: n.generation.clone(),
            hadith_count: n.hadith_count,
        })
        .collect();
    let api_hadiths = hadiths
        .into_iter()
        .map(ApiHadithSearchResult::from)
        .collect();

    Ok(IsnadSearchResponse {
        narrators: api_narrators,
        hadiths: api_hadiths,
        mode: mode.to_string(),
        total,
    })
}

/// Strict-mode helper for [`isnad_search`]: keep only hadiths whose chain has
/// a verifiable `heard_from` edge for every consecutive `(student, teacher)`
/// pair. Sequential — runs `O(narrators - 1)` lookups per candidate hadith.
async fn filter_strict_chains(
    state: &crate::web::AppState,
    narrators: &[Narrator],
    hadiths: Vec<HadithSearchResult>,
) -> Result<Vec<HadithSearchResult>> {
    #[derive(Debug, SurrealValue)]
    struct CountRow {
        c: i64,
    }
    let mut result = Vec::new();
    for h in hadiths {
        let hadith_rid = h.id.clone().unwrap();
        let mut valid = true;
        for pair in narrators.windows(2) {
            let student = pair[0].id.as_ref().unwrap();
            let teacher = pair[1].id.as_ref().unwrap();
            let row: Option<CountRow> = state
                .db
                .query(
                    "SELECT count() AS c FROM heard_from \
                     WHERE in = $student AND out = $teacher AND hadith_ref = $hid \
                     GROUP ALL",
                )
                .bind(("student", student.clone()))
                .bind(("teacher", teacher.clone()))
                .bind(("hid", hadith_rid.clone()))
                .await
                .context("strict chain check failed")?
                .take(0)
                .unwrap_or(None);
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

/// Word-level matn diff between two hadiths (prefers `matn` field, falls back
/// to `text_ar` then `text_en`). Useful for studying narrator paraphrases
/// across variants of the same family.
pub async fn matn_diff(
    state: &AppState,
    a_id: &str,
    b_id: &str,
) -> Result<crate::analysis::matn_diff::MatnDiffResult> {
    let mut res = state
        .db
        .query(format!(
            "SELECT {HADITH_FIELDS} FROM $rid_a; SELECT {HADITH_FIELDS} FROM $rid_b;"
        ))
        .bind(("rid_a", make_record_id("hadith", a_id)))
        .bind(("rid_b", make_record_id("hadith", b_id)))
        .await
        .context("matn diff fetch failed")?;
    let hadith_a: Hadith = res
        .take::<Option<Hadith>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("hadith id not found: {a_id}"))?;
    let hadith_b: Hadith = res
        .take::<Option<Hadith>>(1)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("hadith id not found: {b_id}"))?;

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

    Ok(crate::analysis::matn_diff::diff_matn(
        text_a, text_b, a_id, b_id,
    ))
}
