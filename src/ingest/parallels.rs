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
struct HadithParallelRow {
    hadith_number: i64,
    collection_id: i64,
    parallel_hadith_number: i64,
    parallel_collection_id: i64,
    score: f64,
    source: Option<String>,
}

/// Ingest matn-matched parallel-narration links between hadiths (e.g. Sunan
/// hadiths whose matn closely matches a Bukhari/Muslim narration).
///
/// These records do NOT carry a grade — they are pointers to an equivalent
/// narration that the user can read directly. The UI surfaces them like
/// `similar_hadiths`, distinct from `hadith_grading`.
///
/// Idempotency: deletes all rows with `source = $source` before inserting.
pub async fn ingest_hadith_parallels(db: &Surreal<Db>, file: &str) -> Result<()> {
    crate::db::init_grading_schema(db).await?;

    let raw = std::fs::read_to_string(file)?;
    let rows: Vec<HadithParallelRow> = serde_json::from_str(&raw)?;
    tracing::info!("Loaded {} hadith-parallel rows from {}", rows.len(), file);

    let sources: std::collections::BTreeSet<String> = rows
        .iter()
        .filter_map(|r| r.source.clone())
        .collect();
    for s in &sources {
        db.query("DELETE hadith_parallel WHERE source = $s")
            .bind(("s", s.clone()))
            .await?
            .check()?;
    }
    if !sources.is_empty() {
        tracing::info!("Cleared hadith_parallel rows for sources: {sources:?}");
    }

    let bar = indicatif::ProgressBar::new(rows.len() as u64);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos}/{len} parallels")?,
    );

    let mut inserted = 0u64;
    let mut skipped = 0u64;

    for row in rows {
        let src: Option<HadithIdRow> = db
            .query(
                "SELECT id FROM hadith \
                 WHERE hadith_number = $n AND collection_id = $c LIMIT 1",
            )
            .bind(("n", row.hadith_number))
            .bind(("c", row.collection_id))
            .await?
            .take(0)?;
        let dst: Option<HadithIdRow> = db
            .query(
                "SELECT id FROM hadith \
                 WHERE hadith_number = $n AND collection_id = $c LIMIT 1",
            )
            .bind(("n", row.parallel_hadith_number))
            .bind(("c", row.parallel_collection_id))
            .await?
            .take(0)?;

        match (src, dst) {
            (Some(s), Some(d)) => {
                db.query(
                    "CREATE hadith_parallel SET \
                     hadith_id = $sid, parallel_hadith_id = $did, \
                     score = $score, source = $source",
                )
                .bind(("sid", s.id))
                .bind(("did", d.id))
                .bind(("score", row.score))
                .bind(("source", row.source))
                .await?
                .check()?;
                inserted += 1;
            }
            _ => {
                skipped += 1;
            }
        }
        bar.inc(1);
    }
    bar.finish();
    tracing::info!("Hadith parallels ingest: inserted={inserted}, skipped={skipped}");
    Ok(())
}
