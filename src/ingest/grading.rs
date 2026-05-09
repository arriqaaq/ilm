use anyhow::Result;
use serde::Deserialize;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;

#[derive(Debug, SurrealValue)]
struct HadithIdRow {
    id: surrealdb::types::RecordId,
}

#[derive(Debug, Deserialize)]
struct HadithGradingRow {
    hadith_number: i64,
    collection_id: i64,
    scholar_key: String,
    scholar_ar: String,
    grade: String,
    grade_normalized: Option<String>,
    source_book_id: Option<i64>,
    source_page_index: Option<i64>,
    raw_text: Option<String>,
    notes: Option<String>,
}

/// Load per-hadith verdicts from a JSON file (produced by extract_albani_sunan_grades.py,
/// extract_abi_dawud_grades.py, or extract_jami_cross_refs.py) into the
/// `hadith_grading` table.
///
/// Joins to `hadith` via (hadith_number, collection_id). Rows pointing at a
/// hadith we don't have are skipped (counted, logged at the end).
///
/// Idempotency: before inserting, we delete every existing row whose
/// `source_book_id` appears in the input — so re-running the same extractor
/// yields the same final state without producing duplicates. We do NOT enforce
/// uniqueness on (hadith_id, scholar_key, source_book_id) because Albani
/// regularly issues multiple verdicts per hadith within a single book (e.g.
/// "chain A is sahih, chain B is daif").
pub async fn ingest_hadith_grading(db: &Surreal<Db>, file: &str) -> Result<()> {
    crate::db::init_grading_schema(db).await?;

    let raw = std::fs::read_to_string(file)?;
    let rows: Vec<HadithGradingRow> = serde_json::from_str(&raw)?;
    tracing::info!("Loaded {} hadith-grading rows from {}", rows.len(), file);

    // Idempotency: delete rows for every source_book_id present in the input.
    let mut book_ids: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    for r in &rows {
        if let Some(b) = r.source_book_id {
            book_ids.insert(b);
        }
    }
    for bid in &book_ids {
        db.query("DELETE hadith_grading WHERE source_book_id = $bid")
            .bind(("bid", *bid))
            .await?
            .check()?;
    }
    if !book_ids.is_empty() {
        tracing::info!("Cleared hadith_grading rows for source books: {book_ids:?}");
    }

    let mut inserted = 0u64;
    let mut skipped_no_hadith = 0u64;
    let bar = indicatif::ProgressBar::new(rows.len() as u64);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos}/{len} grades")?,
    );

    for row in rows {
        let hadith_row: Option<HadithIdRow> = db
            .query(
                "SELECT id FROM hadith \
                 WHERE hadith_number = $n AND collection_id = $c LIMIT 1",
            )
            .bind(("n", row.hadith_number))
            .bind(("c", row.collection_id))
            .await?
            .take(0)?;
        let Some(hr) = hadith_row else {
            skipped_no_hadith += 1;
            bar.inc(1);
            continue;
        };

        db.query(
            "CREATE hadith_grading SET \
             hadith_id = $hid, scholar_key = $sk, scholar_ar = $sa, \
             grade = $g, grade_normalized = $gn, \
             source_book_id = $sbi, source_page_index = $spi, \
             raw_text = $rt, notes = $nt",
        )
        .bind(("hid", hr.id))
        .bind(("sk", row.scholar_key))
        .bind(("sa", row.scholar_ar))
        .bind(("g", row.grade))
        .bind(("gn", row.grade_normalized))
        .bind(("sbi", row.source_book_id))
        .bind(("spi", row.source_page_index))
        .bind(("rt", row.raw_text))
        .bind(("nt", row.notes))
        .await?
        .check()?;
        inserted += 1;
        bar.inc(1);
    }
    bar.finish();
    tracing::info!(
        "Hadith grading ingest: inserted={inserted}, no_matching_hadith={skipped_no_hadith}"
    );
    Ok(())
}
