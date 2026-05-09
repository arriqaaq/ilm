//! Quran-domain services.
//!
//! Wraps the curated Quran reads: stats, surah/ayah browse, word morphology,
//! ayah ↔ hadith linkages, root concordance, reciters. Cross-domain semantic
//! search uses [`crate::services::search::search_quran`]; specialized
//! mutashabihat / phrase reads land in a later pass.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use surrealdb::types::SurrealValue;

use crate::models::{ApiHadith, ApiHadithSearchResult, PaginatedResponse};
use crate::quran::models::{
    AYAH_FIELDS, ApiAyah, ApiQuranWord, ApiReciter, ApiSurah, Ayah, QuranStatsResponse, QuranWord,
    Reciter, RootSearchResponse, Surah, SurahDetailResponse,
};
use crate::web::AppState;

/// Composite response for `GET /v1/quran/ayahs/{s}/{a}/hadiths` — curated
/// hadiths (always returned) plus optional semantically-related hadiths.
#[derive(Debug, Serialize)]
pub struct ApiAyahHadiths {
    pub curated: Vec<ApiHadith>,
    pub related: Option<Vec<ApiHadithSearchResult>>,
}

/// Aggregate Quran corpus counts.
pub async fn stats(state: &AppState) -> Result<QuranStatsResponse> {
    #[derive(Debug, SurrealValue)]
    struct CountRow {
        count: i64,
    }
    let mut res = state
        .db
        .query("SELECT count() FROM surah GROUP ALL; SELECT count() FROM ayah GROUP ALL")
        .await
        .context("quran stats query failed")?;
    let surah_count = res
        .take::<Option<CountRow>>(0)
        .unwrap_or(None)
        .map(|c| c.count)
        .unwrap_or(0);
    let ayah_count = res
        .take::<Option<CountRow>>(1)
        .unwrap_or(None)
        .map(|c| c.count)
        .unwrap_or(0);
    Ok(QuranStatsResponse {
        surah_count,
        ayah_count,
    })
}

/// All 114 surahs with metadata.
pub async fn list_surahs(state: &AppState) -> Result<Vec<ApiSurah>> {
    let mut res = state
        .db
        .query("SELECT * FROM surah ORDER BY surah_number ASC")
        .await
        .context("surah list query failed")?;
    let surahs: Vec<Surah> = res.take(0).unwrap_or_default();
    Ok(surahs.into_iter().map(ApiSurah::from).collect())
}

/// One surah plus all its ayahs.
pub async fn get_surah(state: &AppState, number: i64) -> Result<SurahDetailResponse> {
    let mut res = state
        .db
        .query(format!(
            "SELECT * FROM surah WHERE surah_number = $num LIMIT 1; \
             SELECT {AYAH_FIELDS} FROM ayah WHERE surah_number = $num ORDER BY ayah_number ASC"
        ))
        .bind(("num", number))
        .await
        .context("surah detail query failed")?;
    let surah: Surah = res
        .take::<Option<Surah>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("surah not found: {number}"))?;
    let ayahs: Vec<Ayah> = res.take(1).unwrap_or_default();
    Ok(SurahDetailResponse {
        surah: ApiSurah::from(surah),
        ayahs: ayahs.into_iter().map(ApiAyah::from).collect(),
    })
}

/// Distinguishes "surah/ayah not found" so HTTP/MCP can surface 404 /
/// `invalid_request` instead of 500.
pub fn is_not_found(e: &anyhow::Error) -> bool {
    let msg = e.to_string();
    msg.starts_with("surah not found:") || msg.starts_with("ayah not found:")
}

/// Browse ayahs (paginated). When `surah` is set, restrict to that surah and
/// order by ayah_number; otherwise, paginate the whole Quran in order.
pub async fn browse_ayahs(
    state: &AppState,
    surah: Option<i64>,
    page: usize,
    limit: usize,
) -> Result<PaginatedResponse<ApiAyah>> {
    let page = page.max(1);
    let limit = limit.clamp(1, 200);
    let offset = (page - 1) * limit;

    let (sql, has_surah) = if surah.is_some() {
        (
            format!(
                "SELECT {AYAH_FIELDS} FROM ayah WHERE surah_number = $surah \
                 ORDER BY ayah_number ASC LIMIT {limit} START {offset}"
            ),
            true,
        )
    } else {
        (
            format!(
                "SELECT {AYAH_FIELDS} FROM ayah ORDER BY surah_number ASC, ayah_number ASC \
                 LIMIT {limit} START {offset}"
            ),
            false,
        )
    };
    let mut q = state.db.query(&sql);
    if has_surah {
        q = q.bind(("surah", surah.unwrap()));
    }
    let mut res = q.await.context("ayah browse query failed")?;
    let ayahs: Vec<Ayah> = res.take(0).unwrap_or_default();
    let has_more = ayahs.len() == limit;
    Ok(PaginatedResponse {
        data: ayahs.into_iter().map(ApiAyah::from).collect(),
        page,
        limit,
        has_more,
        total: None,
    })
}

/// Word-by-word morphology for one ayah (root, lemma, POS, etc.).
pub async fn get_ayah_words(state: &AppState, surah: i64, ayah: i64) -> Result<Vec<ApiQuranWord>> {
    let mut res = state
        .db
        .query(
            "SELECT * FROM quran_word WHERE surah_number = $s AND ayah_number = $a ORDER BY word_position",
        )
        .bind(("s", surah))
        .bind(("a", ayah))
        .await
        .context("ayah words query failed")?;
    let words: Vec<QuranWord> = res.take(0).unwrap_or_default();
    Ok(words.into_iter().map(ApiQuranWord::from).collect())
}

/// Curated hadiths for one ayah (always) plus optional semantically-related
/// hadiths via the `references_hadith` + vector search hybrid pipeline.
pub async fn get_ayah_hadiths(
    state: &AppState,
    surah: i64,
    ayah: i64,
    include_semantic: bool,
    semantic_limit: usize,
) -> Result<ApiAyahHadiths> {
    let curated = crate::quran::hadith_refs::get_curated_hadiths(&state.db, surah, ayah)
        .await
        .context("curated hadith lookup failed")?;
    let related = if include_semantic {
        let results = crate::quran::hadith_refs::find_semantic_hadiths(
            &state.db,
            surah,
            ayah,
            semantic_limit,
        )
        .await
        .context("semantic hadith lookup failed")?;
        Some(
            results
                .into_iter()
                .map(ApiHadithSearchResult::from)
                .collect(),
        )
    } else {
        None
    };
    Ok(ApiAyahHadiths {
        curated: curated.into_iter().map(ApiHadith::from).collect(),
        related,
    })
}

/// Per-ayah curated hadith counts for one surah. Map key = ayah_number string.
pub async fn get_surah_hadith_counts(state: &AppState, surah: i64) -> Result<HashMap<String, i64>> {
    let counts = crate::quran::hadith_refs::get_hadith_counts(&state.db, surah)
        .await
        .context("surah hadith counts query failed")?;
    Ok(counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect())
}

/// Per-ayah counts of `similar_to` + `shares_phrase` edges (mutashabihat
/// markers used by the Quran reader).
pub async fn get_surah_similar_counts(
    state: &AppState,
    surah: i64,
) -> Result<HashMap<String, i64>> {
    #[derive(Debug, SurrealValue)]
    struct AyahCount {
        ayah_number: i64,
        count: i64,
    }
    let mut res = state
        .db
        .query(
            "LET $ayah_ids = (SELECT id FROM ayah WHERE surah_number = $s); \
             SELECT in.ayah_number AS ayah_number, count() AS count \
             FROM similar_to WHERE in IN $ayah_ids GROUP BY ayah_number; \
             SELECT in.ayah_number AS ayah_number, count() AS count \
             FROM shares_phrase WHERE in IN $ayah_ids GROUP BY ayah_number",
        )
        .bind(("s", surah))
        .await
        .context("surah similar counts query failed")?;
    let similar: Vec<AyahCount> = res.take(1).unwrap_or_default();
    let phrases: Vec<AyahCount> = res.take(2).unwrap_or_default();
    let mut counts: HashMap<String, i64> = HashMap::new();
    for ac in similar.iter().chain(phrases.iter()) {
        *counts.entry(ac.ayah_number.to_string()).or_insert(0) += ac.count;
    }
    Ok(counts)
}

/// Concordance of every Quran occurrence of an Arabic root.
pub async fn search_by_root(state: &AppState, root: &str) -> Result<RootSearchResponse> {
    let mut res = state
        .db
        .query(
            "SELECT * FROM quran_word WHERE root = $root \
             ORDER BY surah_number, ayah_number, word_position",
        )
        .bind(("root", root.to_string()))
        .await
        .context("root search query failed")?;
    let words: Vec<QuranWord> = res.take(0).unwrap_or_default();
    let ayah_count = words
        .iter()
        .map(|w| (w.surah_number, w.ayah_number))
        .collect::<HashSet<_>>()
        .len();
    Ok(RootSearchResponse {
        root: root.to_string(),
        occurrences: words.into_iter().map(ApiQuranWord::from).collect(),
        ayah_count,
    })
}

/// All reciters available for ayah-level audio playback.
pub async fn list_reciters(state: &AppState) -> Result<Vec<ApiReciter>> {
    let mut res = state
        .db
        .query("SELECT * FROM reciter ORDER BY name_en ASC")
        .await
        .context("reciter list query failed")?;
    let reciters: Vec<Reciter> = res.take(0).unwrap_or_default();
    Ok(reciters.into_iter().map(ApiReciter::from).collect())
}
