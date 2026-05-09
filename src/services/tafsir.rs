//! Tafsir-domain services.
//!
//! Tafsir bodies for ayahs are stored as `book_page` rows (one per "section")
//! and indexed by `tafsir_ayah_map` rows that point an `(surah, ayah, book)`
//! triple at a `page_index` in the book. We fetch the mapping then read the
//! page.

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Serialize;
use surrealdb::types::SurrealValue;
use utoipa::ToSchema;

use crate::web::AppState;

/// Default tafsir book id — Ibn Kathir. Other tafsirs (Tabari, etc.) live
/// under their own `book_id` and can be requested explicitly.
pub const DEFAULT_TAFSIR_BOOK_ID: u64 = 23604;

#[derive(Debug, Serialize, ToSchema)]
pub struct TafsirPageRef {
    pub page_index: u64,
    pub heading: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TafsirSurahMappings {
    pub mappings: HashMap<String, TafsirPageRef>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AyahTafsirResponse {
    pub book_id: u64,
    pub surah: u64,
    pub ayah: u64,
    pub page_index: u64,
    pub vol: String,
    pub page_num: u64,
    pub heading: Option<String>,
    pub text: String,
}

#[derive(Debug, SurrealValue)]
struct AyahTafsirMapping {
    page_index: i64,
    heading: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct AyahTafsirPage {
    text: String,
    vol: String,
    page_num: i64,
}

#[derive(Debug, SurrealValue)]
struct MappingRow {
    ayah: i64,
    page_index: i64,
    heading: Option<String>,
}

/// Per-ayah → tafsir-page index for one surah, for one tafsir book. Used by
/// the Quran reader to render heading markers next to each ayah.
pub async fn get_surah_tafsir_pages(
    state: &AppState,
    surah_number: u64,
    book_id: u64,
) -> Result<TafsirSurahMappings> {
    let rows: Vec<MappingRow> = state
        .db
        .query(
            "SELECT ayah, page_index, heading FROM tafsir_ayah_map \
             WHERE surah = $surah AND book_id = $book ORDER BY ayah ASC",
        )
        .bind(("surah", surah_number as i64))
        .bind(("book", book_id as i64))
        .await
        .context("surah tafsir pages query failed")?
        .take(0)
        .unwrap_or_default();
    let mut mappings = HashMap::new();
    for row in rows {
        mappings.insert(
            row.ayah.to_string(),
            TafsirPageRef {
                page_index: row.page_index as u64,
                heading: row.heading,
            },
        );
    }
    Ok(TafsirSurahMappings { mappings })
}

/// Tafsir body for one ayah from one tafsir book. Returns `None` when no
/// mapping exists (caller maps to 404 / invalid_request).
pub async fn get_ayah_tafsir(
    state: &AppState,
    surah: u64,
    ayah: u64,
    book_id: u64,
) -> Result<Option<AyahTafsirResponse>> {
    let mapping: Option<AyahTafsirMapping> = state
        .db
        .query(
            "SELECT page_index, heading FROM tafsir_ayah_map \
             WHERE surah = $s AND ayah = $a AND book_id = $b LIMIT 1",
        )
        .bind(("s", surah as i64))
        .bind(("a", ayah as i64))
        .bind(("b", book_id as i64))
        .await
        .context("tafsir mapping lookup failed")?
        .take(0)
        .unwrap_or(None);

    let Some(mapping) = mapping else {
        return Ok(None);
    };

    let page: Option<AyahTafsirPage> = state
        .db
        .query(
            "SELECT text, vol, page_num FROM book_page \
             WHERE book_id = $b AND page_index = $p LIMIT 1",
        )
        .bind(("b", book_id as i64))
        .bind(("p", mapping.page_index))
        .await
        .context("tafsir page fetch failed")?
        .take(0)
        .unwrap_or(None);

    let Some(page) = page else {
        tracing::warn!(
            "tafsir mapping for {surah}:{ayah} book {book_id} points at missing page {}",
            mapping.page_index
        );
        return Ok(None);
    };

    Ok(Some(AyahTafsirResponse {
        book_id,
        surah,
        ayah,
        page_index: mapping.page_index as u64,
        vol: page.vol,
        page_num: page.page_num as u64,
        heading: mapping.heading,
        text: page.text,
    }))
}
