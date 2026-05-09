//! Scholar / hadith-grading services.
//!
//! Multi-scholar verdicts on individual hadiths. For Bukhari (collection_id=1)
//! and Muslim (collection_id=2) a synthetic "consensus sahih" row is prepended.

use anyhow::{Context, Result};
use surrealdb::types::SurrealValue;

use crate::models::{
    ApiHadithGrading, ApiHadithGradingsResponse, ApiScholar, GradeNormalized, make_record_id,
};
use crate::web::AppState;

#[derive(Debug, SurrealValue)]
struct ScholarRow {
    scholar_key: String,
    scholar_ar: String,
    count: i64,
}

#[derive(Debug, SurrealValue)]
struct CollectionIdRow {
    collection_id: i64,
}

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

/// All scholars known to the grading dataset, with the count of distinct
/// hadith verdicts they have authored.
pub async fn list_scholars(state: &AppState) -> Result<Vec<ApiScholar>> {
    let rows: Vec<ScholarRow> = state
        .db
        .query(
            "SELECT scholar_key, scholar_ar, count() AS count FROM hadith_grading \
             GROUP BY scholar_key, scholar_ar ORDER BY count DESC",
        )
        .await
        .context("scholar list query failed")?
        .take(0)
        .unwrap_or_default();
    Ok(rows
        .into_iter()
        .map(|r| ApiScholar {
            scholar_key: r.scholar_key,
            scholar_ar: r.scholar_ar,
            count: r.count,
        })
        .collect())
}

/// Multi-scholar verdicts for one hadith. For Bukhari/Muslim hadiths a
/// synthetic "consensus sahih" row is prepended to the stored gradings.
pub async fn get_hadith_gradings(
    state: &AppState,
    hadith_id: &str,
) -> Result<ApiHadithGradingsResponse> {
    let hrid = make_record_id("hadith", hadith_id);

    let collection_id: i64 = state
        .db
        .query("SELECT collection_id FROM $rid")
        .bind(("rid", hrid.clone()))
        .await
        .context("hadith collection lookup failed")?
        .take::<Option<CollectionIdRow>>(0)
        .unwrap_or(None)
        .map(|r| r.collection_id)
        .unwrap_or(0);

    let stored: Vec<HadithGradingRow> = state
        .db
        .query(
            "SELECT scholar_key, scholar_ar, grade, grade_normalized, \
             source_book_id, source_page_index, source_vol, source_page_num, \
             raw_text, notes FROM hadith_grading WHERE hadith_id = $rid",
        )
        .bind(("rid", hrid))
        .await
        .context("hadith gradings query failed")?
        .take(0)
        .unwrap_or_default();

    let mut gradings: Vec<ApiHadithGrading> = Vec::with_capacity(stored.len() + 1);

    if collection_id == 1 || collection_id == 2 {
        let (key, ar) = if collection_id == 1 {
            ("bukhari", "البخاري")
        } else {
            ("muslim", "مسلم")
        };
        gradings.push(ApiHadithGrading {
            scholar_key: key.to_string(),
            scholar_ar: ar.to_string(),
            grade: "صحيح".to_string(),
            grade_normalized: Some(GradeNormalized::Sahih),
            source_book_id: None,
            source_page_index: None,
            source_vol: None,
            source_page_num: None,
            raw_text: None,
            notes: Some("consensus sahih".to_string()),
        });
    }

    for r in stored {
        let grade_normalized = r
            .grade_normalized
            .as_deref()
            .and_then(GradeNormalized::parse);
        gradings.push(ApiHadithGrading {
            scholar_key: r.scholar_key,
            scholar_ar: r.scholar_ar,
            grade: r.grade,
            grade_normalized,
            source_book_id: r.source_book_id,
            source_page_index: r.source_page_index,
            source_vol: r.source_vol,
            source_page_num: r.source_page_num,
            raw_text: r.raw_text,
            notes: r.notes,
        });
    }

    Ok(ApiHadithGradingsResponse {
        hadith_id: hadith_id.to_string(),
        gradings,
    })
}
