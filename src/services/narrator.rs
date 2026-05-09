//! Narrator-domain services.
//!
//! Currently absorbs the structured "tools" surface used by the agentic RAG
//! pipeline (resolve, count, info, teachers, students, hadiths, isnad search,
//! common narrators). HTTP-handler bodies for narrator browsing/listing will
//! land here in a later phase.

use std::collections::HashSet;

use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;
use crate::models::{
    ApiHadith, ApiNarrator, ApiNarratorDetail, ApiNarratorIsnadRole, ApiNarratorSearchResult,
    ApiNarratorWithCount, CommonNarratorsResponse, HADITH_FIELDS, Hadith, HadithSearchResult,
    Narrator, NarratorWithCount, PaginatedResponse, make_record_id, record_id_key_string,
    record_id_string,
};
use crate::web::AppState;

/// Output from a structured tool execution, ready for both SSE and LLM context.
pub struct ToolOutput {
    /// Formatted text for the LLM system prompt.
    pub context: String,
    /// Narrator sources for the SSE sources event.
    pub narrator_sources: Vec<ApiNarratorSource>,
    /// Hadith sources for the SSE sources event.
    pub hadith_sources: Vec<HadithSearchResult>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ApiNarratorSource {
    pub id: String,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub generation: Option<String>,
    pub hadith_count: Option<i64>,
    pub kunya: Option<String>,
    pub bio: Option<String>,
    pub death_year: Option<i64>,
    pub teachers: Vec<NarratorBrief>,
    pub students: Vec<NarratorBrief>,
}

#[derive(Debug, Serialize, Clone)]
pub struct NarratorBrief {
    pub id: String,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub generation: Option<String>,
}

impl From<Narrator> for NarratorBrief {
    fn from(n: Narrator) -> Self {
        Self {
            id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n.name_ar,
            name_en: n.name_en,
            generation: n.generation,
        }
    }
}

// ── Narrator Resolution ──

/// Fuzzy-resolve a narrator name to the best-matching narrator record.
pub async fn resolve_narrator(db: &Surreal<Db>, name: &str) -> Result<Option<Narrator>> {
    let lower = name.to_lowercase();
    let slug = crate::quran::ingest::strip_arabic_diacritics(name);

    // Multi-signal search: name_en, name_ar, kunya, aliases, search_name slug
    let sql = "\
        SELECT * FROM narrator WHERE \
            string::lowercase(name_en) CONTAINS string::lowercase($q) \
            OR name_ar CONTAINS $q \
            OR kunya CONTAINS $q \
            OR search_name CONTAINS $slug \
            OR $q INSIDE aliases \
        ORDER BY hadith_count DESC \
        LIMIT 5";

    let mut res = db
        .query(sql)
        .bind(("q", lower))
        .bind(("slug", slug))
        .await?;

    let matches: Vec<Narrator> = res.take(0).unwrap_or_default();
    Ok(matches.into_iter().next())
}

// ── Tool Functions ──

/// Count hadiths narrated by a narrator, optionally filtered by book name.
pub async fn count_hadiths(
    db: &Surreal<Db>,
    narrator: &Narrator,
    book: Option<&str>,
) -> Result<ToolOutput> {
    let nid = narrator.id.as_ref().unwrap();
    let name = narrator.name_ar.as_deref().unwrap_or(&narrator.name_en);

    let (count, book_label) = if let Some(book_name) = book {
        // Resolve book name → book_id first, then use indexed int comparison.
        // hadith.book_id has hadith_book index; narrates.in has narrates_in_idx.
        #[derive(Debug, SurrealValue)]
        struct BookRow {
            book_number: i64,
            name_en: String,
        }
        let mut book_res = db
            .query(
                "SELECT book_number, name_en FROM book \
                 WHERE string::lowercase(name_en) CONTAINS string::lowercase($name) \
                 LIMIT 1",
            )
            .bind(("name", book_name.to_string()))
            .await?;
        let resolved_book: Option<BookRow> = book_res.take(0).unwrap_or(None);

        if let Some(bk) = resolved_book {
            #[derive(Debug, SurrealValue)]
            struct CountRow {
                count: i64,
            }
            // Use book_id (int) comparison — out.book_id uses hadith_book index
            let sql = "SELECT count() AS count FROM narrates \
                       WHERE in = $nid AND out.book_id = $book_id \
                       GROUP ALL";
            let mut res = db
                .query(sql)
                .bind(("nid", nid.clone()))
                .bind(("book_id", bk.book_number))
                .await?;
            let row: Option<CountRow> = res.take(0).unwrap_or(None);
            (row.map(|r| r.count).unwrap_or(0), Some(bk.name_en))
        } else {
            // Book not found — fall back to string match on book_name
            #[derive(Debug, SurrealValue)]
            struct CountRow {
                count: i64,
            }
            let sql = "SELECT count() AS count FROM narrates \
                       WHERE in = $nid AND out.book_name CONTAINS $book \
                       GROUP ALL";
            let mut res = db
                .query(sql)
                .bind(("nid", nid.clone()))
                .bind(("book", book_name.to_string()))
                .await?;
            let row: Option<CountRow> = res.take(0).unwrap_or(None);
            (
                row.map(|r| r.count).unwrap_or(0),
                Some(book_name.to_string()),
            )
        }
    } else {
        // Use pre-computed hadith_count
        (narrator.hadith_count.unwrap_or(0), None)
    };

    let mut context = "## Narrator Hadith Count\n\n".to_string();
    context.push_str(&format!("Narrator: {} ({})\n", name, narrator.name_en));
    if let Some(generation) = &narrator.generation {
        context.push_str(&format!("Generation (Tabaqah): {generation}\n"));
    }
    if let Some(ref book_label) = book_label {
        context.push_str(&format!("Hadiths narrated in {book_label}: {count}\n"));
    } else {
        context.push_str(&format!("Total hadiths narrated: {count}\n"));
    }

    let source = narrator_to_source(narrator, vec![], vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: vec![],
    })
}

/// Get full narrator bio info.
pub async fn narrator_info(_db: &Surreal<Db>, narrator: &Narrator) -> Result<ToolOutput> {
    let mut context = "## Narrator Information\n\n".to_string();
    context.push_str(&format!(
        "Name (Arabic): {}\n",
        narrator.name_ar.as_deref().unwrap_or("N/A")
    ));
    context.push_str(&format!("Name (English): {}\n", narrator.name_en));
    if let Some(kunya) = &narrator.kunya {
        context.push_str(&format!("Kunya: {kunya}\n"));
    }
    if let Some(generation) = &narrator.generation {
        context.push_str(&format!("Generation (Tabaqah): {generation}\n"));
    }
    if let Some(death) = narrator.death_year {
        context.push_str(&format!("Death year: {death} AH\n"));
    }
    if let Some(bio) = &narrator.bio {
        context.push_str(&format!("Biography: {bio}\n"));
    }
    if let Some(count) = narrator.hadith_count {
        context.push_str(&format!("Total hadiths narrated: {count}\n"));
    }
    if let Some(aliases) = &narrator.aliases
        && !aliases.is_empty()
    {
        context.push_str(&format!("Also known as: {}\n", aliases.join(", ")));
    }
    let source = narrator_to_source(narrator, vec![], vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: vec![],
    })
}

// ── Helper types for graph queries ──

#[derive(Debug, SurrealValue)]
struct TeachersResult {
    teachers: Vec<Narrator>,
}

#[derive(Debug, SurrealValue)]
struct StudentsResult {
    students: Vec<Narrator>,
}

/// Get narrator's teachers (who they heard from).
pub async fn narrator_teachers(db: &Surreal<Db>, narrator: &Narrator) -> Result<ToolOutput> {
    let nid = narrator.id.as_ref().unwrap();
    let name = narrator.name_ar.as_deref().unwrap_or(&narrator.name_en);

    let mut res = db
        .query(
            "SELECT array::distinct(array::filter(->heard_from->narrator.*, |$v| $v IS NOT NONE)) AS teachers FROM $nid",
        )
        .bind(("nid", nid.clone()))
        .await?;

    let result: Option<TeachersResult> = res.take(0).unwrap_or(None);
    let teachers = result.map(|r| r.teachers).unwrap_or_default();

    let mut context = format!("## Teachers of {name} ({})\n\n", narrator.name_en);
    context.push_str(&format!(
        "{name} had {} known teacher(s):\n\n",
        teachers.len()
    ));
    for t in &teachers {
        let tname = t.name_ar.as_deref().unwrap_or(&t.name_en);
        context.push_str(&format!("- {} ({})", tname, t.name_en));
        if let Some(generation) = &t.generation {
            context.push_str(&format!(", generation {generation}"));
        }
        context.push('\n');
    }

    let teacher_briefs: Vec<NarratorBrief> =
        teachers.iter().cloned().map(NarratorBrief::from).collect();
    let source = narrator_to_source(narrator, teacher_briefs, vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: vec![],
    })
}

/// Get narrator's students (who heard from them).
pub async fn narrator_students(db: &Surreal<Db>, narrator: &Narrator) -> Result<ToolOutput> {
    let nid = narrator.id.as_ref().unwrap();
    let name = narrator.name_ar.as_deref().unwrap_or(&narrator.name_en);

    let mut res = db
        .query(
            "SELECT array::distinct(array::filter(<-heard_from<-narrator.*, |$v| $v IS NOT NONE)) AS students FROM $nid",
        )
        .bind(("nid", nid.clone()))
        .await?;

    let result: Option<StudentsResult> = res.take(0).unwrap_or(None);
    let students = result.map(|r| r.students).unwrap_or_default();

    let mut context = format!("## Students of {name} ({})\n\n", narrator.name_en);
    context.push_str(&format!(
        "{name} had {} known student(s):\n\n",
        students.len()
    ));
    for s in &students {
        let sname = s.name_ar.as_deref().unwrap_or(&s.name_en);
        context.push_str(&format!("- {} ({})", sname, s.name_en));
        if let Some(generation) = &s.generation {
            context.push_str(&format!(", generation {generation}"));
        }
        context.push('\n');
    }

    let student_briefs: Vec<NarratorBrief> =
        students.iter().cloned().map(NarratorBrief::from).collect();
    let source = narrator_to_source(narrator, vec![], student_briefs);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: vec![],
    })
}

/// Get sample hadiths narrated by a narrator.
pub async fn narrator_hadiths(
    db: &Surreal<Db>,
    narrator: &Narrator,
    limit: usize,
) -> Result<ToolOutput> {
    let nid = narrator.id.as_ref().unwrap();
    let name = narrator.name_ar.as_deref().unwrap_or(&narrator.name_en);
    let total = narrator.hadith_count.unwrap_or(0);

    // Query narrates edges directly with LIMIT — avoids fetching all hadiths into memory.
    // Uses narrates_in_idx on `in`.
    let sql = format!(
        "SELECT out.id AS id, out.hadith_number AS hadith_number, out.book_id AS book_id, \
         out.text_ar AS text_ar, out.text_en AS text_en, out.narrator_text AS narrator_text \
         FROM narrates WHERE in = $nid LIMIT {limit}"
    );

    let mut res = db.query(&sql).bind(("nid", nid.clone())).await?;
    let sample: Vec<HadithSearchResult> = res.take(0).unwrap_or_default();

    let mut context = format!("## Hadiths narrated by {name} ({})\n\n", narrator.name_en);
    context.push_str(&format!(
        "Showing {} of {} total hadiths:\n\n",
        sample.len(),
        total
    ));
    for h in &sample {
        context.push_str(&format!("Hadith #{}\n", h.hadith_number));
        if let Some(text) = h.text_en.as_deref().or(h.text_ar.as_deref()) {
            let truncated = if text.len() > 300 {
                &text[..text.floor_char_boundary(300)]
            } else {
                text
            };
            context.push_str(&format!("{truncated}\n\n"));
        }
    }

    let source = narrator_to_source(narrator, vec![], vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: sample,
    })
}

/// Find hadiths where all specified narrators appear in the chain.
pub async fn isnad_search_tool(
    db: &Surreal<Db>,
    narrators: &[Narrator],
    ordered: bool,
    limit: usize,
) -> Result<ToolOutput> {
    let narrator_rids: Vec<RecordId> = narrators.iter().filter_map(|n| n.id.clone()).collect();
    let narrator_count = narrator_rids.len() as i64;

    let sql = format!(
        "LET $matched = (SELECT VALUE out FROM (\
            SELECT out, count() AS c FROM narrates \
            WHERE in IN $narrator_ids \
            GROUP BY out\
        ) WHERE c = $narrator_count); \
        SELECT id, hadith_number, collection_id, text_ar, text_en, narrator_text \
        FROM hadith WHERE id IN $matched LIMIT {limit}"
    );

    let mut res = db
        .query(&sql)
        .bind(("narrator_ids", narrator_rids))
        .bind(("narrator_count", narrator_count))
        .await?;
    let mut hadiths: Vec<HadithSearchResult> = res.take(1).unwrap_or_default();

    // For ordered mode, verify heard_from edges between consecutive narrators
    if ordered && narrators.len() >= 2 {
        #[derive(Debug, SurrealValue)]
        struct StrictCount {
            c: i64,
        }
        let mut filtered = Vec::new();
        for h in hadiths {
            let hrid = h.id.clone().unwrap();
            let mut valid = true;
            for pair in narrators.windows(2) {
                let student = pair[0].id.as_ref().unwrap();
                let teacher = pair[1].id.as_ref().unwrap();
                let mut check = db
                    .query(
                        "SELECT count() AS c FROM heard_from \
                         WHERE in = $student AND out = $teacher AND hadith_ref = $hid \
                         GROUP ALL",
                    )
                    .bind(("student", student.clone()))
                    .bind(("teacher", teacher.clone()))
                    .bind(("hid", hrid.clone()))
                    .await?;
                let row: Option<StrictCount> = check.take(0).unwrap_or(None);
                if row.map(|r| r.c).unwrap_or(0) == 0 {
                    valid = false;
                    break;
                }
            }
            if valid {
                filtered.push(h);
            }
        }
        hadiths = filtered;
    }

    let names: Vec<String> = narrators
        .iter()
        .map(|n| n.name_ar.as_deref().unwrap_or(&n.name_en).to_string())
        .collect();
    let mode_label = if ordered {
        "ordered chain"
    } else {
        "any order"
    };

    let mut context = format!(
        "## Isnad Search Results ({})\n\nNarrators: {}\n",
        mode_label,
        names.join(", ")
    );
    context.push_str(&format!(
        "Found {} hadiths where all narrators appear in the chain:\n\n",
        hadiths.len()
    ));
    for h in &hadiths {
        context.push_str(&format!(
            "Hadith #{} (Book {})\n",
            h.hadith_number, h.collection_id
        ));
        if let Some(text) = h.text_en.as_deref().or(h.text_ar.as_deref()) {
            let truncated = if text.len() > 300 {
                &text[..text.floor_char_boundary(300)]
            } else {
                text
            };
            context.push_str(&format!("{truncated}\n\n"));
        }
    }

    let narrator_sources: Vec<ApiNarratorSource> = narrators
        .iter()
        .map(|n| narrator_to_source(n, vec![], vec![]))
        .collect();

    Ok(ToolOutput {
        context,
        narrator_sources,
        hadith_sources: hadiths,
    })
}

/// Find narrators common to two narrators' chains.
pub async fn common_narrators_tool(
    db: &Surreal<Db>,
    n1: &Narrator,
    n2: &Narrator,
) -> Result<ToolOutput> {
    let nid1 = n1.id.as_ref().unwrap();
    let nid2 = n2.id.as_ref().unwrap();
    let name1 = n1.name_ar.as_deref().unwrap_or(&n1.name_en);
    let name2 = n2.name_ar.as_deref().unwrap_or(&n2.name_en);

    // Get hadith sets for each narrator
    #[derive(Debug, SurrealValue)]
    struct OutId {
        out: RecordId,
    }
    let mut res = db
        .query(
            "SELECT out FROM narrates WHERE in = $nid1; \
             SELECT out FROM narrates WHERE in = $nid2",
        )
        .bind(("nid1", nid1.clone()))
        .bind(("nid2", nid2.clone()))
        .await?;

    let set1: Vec<OutId> = res.take(0).unwrap_or_default();
    let set2: Vec<OutId> = res.take(1).unwrap_or_default();

    let ids2: HashSet<String> = set2.iter().map(|r| record_id_string(&r.out)).collect();
    let shared: Vec<RecordId> = set1
        .into_iter()
        .filter(|r| ids2.contains(&record_id_string(&r.out)))
        .map(|r| r.out)
        .collect();

    let mut context = format!("## Common Narrators between {} and {}\n\n", name1, name2);

    if shared.is_empty() {
        context.push_str("No shared hadiths found between these narrators.\n");
        let src1 = narrator_to_source(n1, vec![], vec![]);
        let src2 = narrator_to_source(n2, vec![], vec![]);
        return Ok(ToolOutput {
            context,
            narrator_sources: vec![src1, src2],
            hadith_sources: vec![],
        });
    }

    context.push_str(&format!("Found {} shared hadiths.\n\n", shared.len()));

    // Find other narrators in those shared hadiths
    #[derive(Debug, SurrealValue)]
    struct NarratorRef {
        narrator: RecordId,
    }
    let mut res = db
        .query(
            "SELECT in AS narrator FROM narrates \
             WHERE out IN $shared AND in != $nid1 AND in != $nid2",
        )
        .bind(("shared", shared))
        .bind(("nid1", nid1.clone()))
        .bind(("nid2", nid2.clone()))
        .await?;

    let refs: Vec<NarratorRef> = res.take(0).unwrap_or_default();

    let mut seen = HashSet::new();
    let unique_ids: Vec<RecordId> = refs
        .into_iter()
        .filter(|r| seen.insert(record_id_string(&r.narrator)))
        .map(|r| r.narrator)
        .collect();

    let common_narrators: Vec<Narrator> = if unique_ids.is_empty() {
        vec![]
    } else {
        let mut res = db
            .query("SELECT * FROM narrator WHERE id IN $ids ORDER BY hadith_count DESC LIMIT 20")
            .bind(("ids", unique_ids))
            .await?;
        res.take(0).unwrap_or_default()
    };

    context.push_str(&format!(
        "Found {} narrators common to both chains:\n\n",
        common_narrators.len()
    ));
    for n in &common_narrators {
        let cname = n.name_ar.as_deref().unwrap_or(&n.name_en);
        context.push_str(&format!("- {} ({})", cname, n.name_en));
        if let Some(generation) = &n.generation {
            context.push_str(&format!(", generation {generation}"));
        }
        context.push('\n');
    }

    let src1 = narrator_to_source(n1, vec![], vec![]);
    let src2 = narrator_to_source(n2, vec![], vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![src1, src2],
        hadith_sources: vec![],
    })
}

// ── HTTP-shaped read services (also drive matching MCP tools) ──────────────

fn narrator_with_count_to_api(n: NarratorWithCount) -> ApiNarratorWithCount {
    ApiNarratorWithCount {
        id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
        name_ar: n.name_ar,
        name_en: n.name_en,
        generation: n.generation,
        bio: n.bio,
        kunya: n.kunya,
        death_year: n.death_year,
        hadith_count: n.hadith_count.unwrap_or(0),
    }
}

/// Paginated narrator list with optional free-text and generation filters.
/// Sorted by `hadith_count DESC` so the most-cited narrators surface first.
pub async fn list(
    state: &AppState,
    q: Option<&str>,
    generation: Option<&str>,
    page: usize,
    limit: usize,
) -> Result<PaginatedResponse<ApiNarratorWithCount>> {
    let page = page.max(1);
    let limit = limit.clamp(1, 100);
    let offset = (page - 1) * limit;

    let mut conditions: Vec<&str> = Vec::new();
    if q.is_some() {
        conditions.push(
            "(string::lowercase(name_en) CONTAINS string::lowercase($q) OR name_ar CONTAINS $q)",
        );
    }
    if generation.is_some() {
        conditions.push("generation = $generation");
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };
    let sql = format!(
        "SELECT * FROM narrator {where_clause} \
         ORDER BY hadith_count DESC LIMIT $limit START $offset"
    );

    let mut query = state.db.query(&sql);
    if let Some(qv) = q {
        query = query.bind(("q", qv.to_string()));
    }
    if let Some(g) = generation {
        query = query.bind(("generation", g.to_string()));
    }
    query = query.bind(("limit", limit)).bind(("offset", offset));

    let rows: Vec<NarratorWithCount> = query
        .await
        .context("narrator list query failed")?
        .take(0)
        .unwrap_or_default();
    let has_more = rows.len() == limit;
    Ok(PaginatedResponse {
        data: rows.into_iter().map(narrator_with_count_to_api).collect(),
        page,
        limit,
        has_more,
        total: None,
    })
}

/// Narrator detail: bio + sample hadiths + (deduped) teachers + (deduped)
/// students. Returns `Err` with a "not found" marker when the id is missing.
pub async fn get_detail(state: &AppState, id: &str) -> Result<ApiNarratorDetail> {
    let nrid = make_record_id("narrator", id);

    let mut res = state
        .db
        .query(format!(
            "SELECT * FROM $rid; \
             SELECT ->narrates->hadith.{{{HADITH_FIELDS}}} AS hadiths FROM $rid; \
             SELECT array::distinct(array::filter(->heard_from->narrator.*, |$v| $v IS NOT NONE)) AS teachers FROM $rid; \
             SELECT array::distinct(array::filter(<-heard_from<-narrator.*, |$v| $v IS NOT NONE)) AS students FROM $rid;"
        ))
        .bind(("rid", nrid))
        .await
        .context("narrator detail query failed")?;

    let narrator: Narrator = res
        .take::<Option<Narrator>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("narrator not found: {id}"))?;

    #[derive(Debug, SurrealValue)]
    struct HadithsRow {
        hadiths: Vec<Hadith>,
    }
    #[derive(Debug, SurrealValue)]
    struct TeachersRow {
        teachers: Vec<Narrator>,
    }
    #[derive(Debug, SurrealValue)]
    struct StudentsRow {
        students: Vec<Narrator>,
    }
    let hadiths = res
        .take::<Option<HadithsRow>>(1)
        .unwrap_or(None)
        .map(|r| r.hadiths)
        .unwrap_or_default();
    let teachers = dedup_by_id(
        res.take::<Option<TeachersRow>>(2)
            .unwrap_or(None)
            .map(|r| r.teachers)
            .unwrap_or_default(),
    );
    let students = dedup_by_id(
        res.take::<Option<StudentsRow>>(3)
            .unwrap_or(None)
            .map(|r| r.students)
            .unwrap_or_default(),
    );

    Ok(ApiNarratorDetail {
        narrator: ApiNarrator::from(narrator),
        hadiths: hadiths.into_iter().map(ApiHadith::from).collect(),
        teachers: teachers.into_iter().map(ApiNarrator::from).collect(),
        students: students.into_iter().map(ApiNarrator::from).collect(),
    })
}

/// Distinguish narrator-not-found errors so HTTP/MCP can surface 404 /
/// `invalid_request` instead of 500.
pub fn is_not_found(e: &anyhow::Error) -> bool {
    e.to_string().starts_with("narrator not found:")
}

/// Cytoscape-shaped narrator network — the centre narrator plus their
/// immediate teachers (incoming) and students (outgoing). Capped at 25 nodes
/// per side to keep the graph renderable.
pub async fn get_graph(state: &AppState, id: &str) -> Result<crate::models::GraphData> {
    use crate::models::{GraphData, GraphEdge, GraphEdgeData, GraphNode, GraphNodeData};

    let nrid = make_record_id("narrator", id);

    #[derive(Debug, SurrealValue)]
    struct TeachersRow {
        teachers: Vec<Narrator>,
    }
    #[derive(Debug, SurrealValue)]
    struct StudentsRow {
        students: Vec<Narrator>,
    }

    let mut res = state
        .db
        .query(
            "SELECT * FROM $rid; \
             SELECT array::distinct(array::filter(->heard_from->narrator.*, |$v| $v IS NOT NONE)) AS teachers FROM $rid; \
             SELECT array::distinct(array::filter(<-heard_from<-narrator.*, |$v| $v IS NOT NONE)) AS students FROM $rid;",
        )
        .bind(("rid", nrid))
        .await
        .context("narrator graph query failed")?;

    let narrator: Option<Narrator> = res.take(0).unwrap_or(None);
    let teachers = res
        .take::<Option<TeachersRow>>(1)
        .unwrap_or(None)
        .map(|r| r.teachers)
        .unwrap_or_default();
    let students = res
        .take::<Option<StudentsRow>>(2)
        .unwrap_or(None)
        .map(|r| r.students)
        .unwrap_or_default();

    let teachers = dedup_by_id(teachers);
    let students = dedup_by_id(students);
    let total_teachers = teachers.len();
    let total_students = students.len();

    const MAX_GRAPH_NODES: usize = 25;
    let teachers: Vec<_> = teachers.into_iter().take(MAX_GRAPH_NODES).collect();
    let students: Vec<_> = students.into_iter().take(MAX_GRAPH_NODES).collect();

    let mut graph = GraphData {
        nodes: Vec::new(),
        edges: Vec::new(),
        total_teachers: Some(total_teachers),
        total_students: Some(total_students),
    };

    let Some(narrator) = narrator else {
        return Ok(graph);
    };
    let Some(nid) = &narrator.id else {
        return Ok(graph);
    };
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

    Ok(graph)
}

/// Find narrators who appear in chains of hadiths shared by both `a` and `b`.
/// Returns the two anchor narrators plus up to 50 common third-party narrators
/// ordered by hadith_count DESC. Errors with `narrator not found:<id>` (404)
/// when either anchor is missing.
pub async fn list_common(state: &AppState, a: &str, b: &str) -> Result<CommonNarratorsResponse> {
    use std::collections::HashSet;
    use surrealdb::types::RecordId;

    let nrid1 = make_record_id("narrator", a);
    let nrid2 = make_record_id("narrator", b);

    // 1. Resolve both anchors.
    let mut res = state
        .db
        .query("SELECT * FROM $rid1; SELECT * FROM $rid2")
        .bind(("rid1", nrid1))
        .bind(("rid2", nrid2))
        .await
        .context("common narrators anchor lookup failed")?;
    let n1: Narrator = res
        .take::<Option<Narrator>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("narrator not found: {a}"))?;
    let n2: Narrator = res
        .take::<Option<Narrator>>(1)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("narrator not found: {b}"))?;
    let nid1 = n1.id.as_ref().unwrap().clone();
    let nid2 = n2.id.as_ref().unwrap().clone();

    let to_search = |n: &Narrator| ApiNarratorSearchResult {
        id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
        name_ar: n.name_ar.clone(),
        name_en: n.name_en.clone(),
        generation: n.generation.clone(),
        hadith_count: n.hadith_count,
    };

    // 2. Hadith sets per anchor; intersect.
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
        .context("common narrators hadith-set query failed")?;
    let set1: Vec<OutId> = res.take(0).unwrap_or_default();
    let set2: Vec<OutId> = res.take(1).unwrap_or_default();
    let ids2: HashSet<String> = set2.iter().map(|r| record_id_string(&r.out)).collect();
    let shared: Vec<RecordId> = set1
        .into_iter()
        .filter(|r| ids2.contains(&record_id_string(&r.out)))
        .map(|r| r.out)
        .collect();

    if shared.is_empty() {
        return Ok(CommonNarratorsResponse {
            narrator1: to_search(&n1),
            narrator2: to_search(&n2),
            common: vec![],
        });
    }

    // 3. Other narrators present in those shared hadiths.
    #[derive(Debug, SurrealValue)]
    struct NarratorRef {
        narrator: RecordId,
    }
    let refs: Vec<NarratorRef> = state
        .db
        .query(
            "SELECT in AS narrator FROM narrates \
             WHERE out IN $shared AND in != $nid1 AND in != $nid2",
        )
        .bind(("shared", shared))
        .bind(("nid1", nid1))
        .bind(("nid2", nid2))
        .await
        .context("common narrators traversal query failed")?
        .take(0)
        .unwrap_or_default();

    let mut seen = HashSet::new();
    let unique_ids: Vec<RecordId> = refs
        .into_iter()
        .filter(|r| seen.insert(record_id_string(&r.narrator)))
        .map(|r| r.narrator)
        .collect();

    let common_rows: Vec<NarratorWithCount> = if unique_ids.is_empty() {
        vec![]
    } else {
        state
            .db
            .query("SELECT * FROM narrator WHERE id IN $ids ORDER BY hadith_count DESC LIMIT 50")
            .bind(("ids", unique_ids))
            .await
            .context("common narrators detail query failed")?
            .take(0)
            .unwrap_or_default()
    };

    Ok(CommonNarratorsResponse {
        narrator1: to_search(&n1),
        narrator2: to_search(&n2),
        common: common_rows
            .into_iter()
            .map(narrator_with_count_to_api)
            .collect(),
    })
}

/// How often this narrator appears as an isnad pivot or bottleneck across
/// analyzed families. Use alongside `services::family::analyze_family_mustalah`
/// to reason about isnad reliability.
pub async fn get_isnad_role(state: &AppState, id: &str) -> Result<ApiNarratorIsnadRole> {
    #[derive(Debug, SurrealValue)]
    struct PivotInfo {
        family: Option<surrealdb::types::RecordId>,
        is_bottleneck: Option<bool>,
    }
    let rows: Vec<PivotInfo> = state
        .db
        .query("SELECT family, is_bottleneck FROM narrator_pivot WHERE narrator = $nid")
        .bind(("nid", make_record_id("narrator", id)))
        .await
        .context("narrator isnad role query failed")?
        .take(0)
        .unwrap_or_default();

    let pivot_count = rows.len();
    let bottleneck_count = rows
        .iter()
        .filter(|r| r.is_bottleneck == Some(true))
        .count();
    let families: Vec<String> = rows
        .iter()
        .filter_map(|r| r.family.as_ref().map(record_id_key_string))
        .collect();

    Ok(ApiNarratorIsnadRole {
        narrator_id: id.to_string(),
        pivot_family_count: pivot_count,
        bottleneck_family_count: bottleneck_count,
        families,
    })
}

fn dedup_by_id(narrators: Vec<Narrator>) -> Vec<Narrator> {
    let mut seen = HashSet::new();
    narrators
        .into_iter()
        .filter(|n| {
            n.id.as_ref()
                .map(|id| seen.insert(record_id_string(id)))
                .unwrap_or(false)
        })
        .collect()
}

/// Typeahead autocomplete over narrator names — searches `name_en`, `name_ar`,
/// `kunya`, `aliases`, and the `search_name` slug. Sorted by `hadith_count`.
pub async fn autocomplete(
    state: &AppState,
    q: &str,
    limit: usize,
) -> Result<Vec<ApiNarratorWithCount>> {
    let q = q.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let limit = limit.clamp(1, 50);
    let slug = crate::quran::ingest::strip_arabic_diacritics(q);

    let sql = "SELECT * FROM narrator \
        WHERE string::lowercase(name_en) CONTAINS string::lowercase($q) \
           OR name_ar CONTAINS $q \
           OR kunya CONTAINS $q \
           OR search_name CONTAINS $slug \
           OR $q INSIDE aliases \
        ORDER BY hadith_count DESC \
        LIMIT $limit";

    let rows: Vec<NarratorWithCount> = state
        .db
        .query(sql)
        .bind(("q", q.to_string()))
        .bind(("slug", slug))
        .bind(("limit", limit))
        .await
        .context("narrator autocomplete query failed")?
        .take(0)
        .unwrap_or_default();
    Ok(rows.into_iter().map(narrator_with_count_to_api).collect())
}

// ── Helpers ──

fn narrator_to_source(
    n: &Narrator,
    teachers: Vec<NarratorBrief>,
    students: Vec<NarratorBrief>,
) -> ApiNarratorSource {
    ApiNarratorSource {
        id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
        name_ar: n.name_ar.clone(),
        name_en: n.name_en.clone(),
        generation: n.generation.clone(),
        hadith_count: n.hadith_count,
        kunya: n.kunya.clone(),
        bio: n.bio.clone(),
        death_year: n.death_year,
        teachers,
        students,
    }
}
