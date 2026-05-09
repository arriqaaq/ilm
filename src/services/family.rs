//! Family / mustalah-analysis services.
//!
//! Hadith families are clusters of variants computed from embedding similarity
//! ([`crate::analysis::family`]); mustalah analysis ([`crate::analysis::mustalah`])
//! produces structural transmission stats per family (breadth, pivots, defects).

use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use surrealdb::types::{RecordId, SurrealValue};

use crate::models::{
    ApiHadith, ApiHadithFamily, HADITH_FIELDS, Hadith, HadithFamily, PaginatedResponse,
    make_record_id, record_id_key_string,
};
use crate::web::AppState;

/// Composite response for `GET /v1/families/{id}` — the family record plus
/// the hadith variants that make it up.
#[derive(Debug, Serialize)]
pub struct ApiFamilyDetail {
    pub family: ApiHadithFamily,
    pub hadiths: Vec<ApiHadith>,
}

/// Structural mustalah analysis for one family — the analysis row, all chain
/// assessments, and the narrator pivots ordered by bundle coverage.
#[derive(Debug, Serialize)]
pub struct ApiMustalahFamilyAnalysis {
    pub analysis: Option<MustalahAnalysisRow>,
    pub chains: Vec<MustalahChainRow>,
    pub pivots: Vec<MustalahPivotRow>,
}

#[derive(Debug, Serialize, SurrealValue)]
pub struct MustalahAnalysisRow {
    pub breadth_class: Option<String>,
    pub min_breadth: Option<i64>,
    pub bottleneck_tabaqah: Option<i64>,
    pub chain_count: Option<i64>,
    pub ilal_flags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct MustalahChainRow {
    pub variant_id: String,
    pub narrator_count: Option<i64>,
    pub has_chronology_conflict: Option<bool>,
    pub narrator_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct MustalahPivotRow {
    pub narrator_id: String,
    pub bundle_coverage: Option<f64>,
    pub fan_out: Option<i64>,
    pub collector_diversity: Option<i64>,
    pub bypass_count: Option<i64>,
    pub is_bottleneck: Option<bool>,
}

/// Aggregate mustalah counts across all analyzed families — useful for a
/// dashboard / quick overview.
#[derive(Debug, Serialize)]
pub struct ApiMustalahStats {
    pub family_count: i64,
    pub analyzed_count: i64,
    pub mutawatir_count: i64,
    pub mashhur_count: i64,
    pub aziz_count: i64,
    pub gharib_count: i64,
}

/// Paginated hadith family list, sorted by `variant_count DESC` (largest
/// clusters first — they are typically the most-discussed across collections).
pub async fn list(
    state: &AppState,
    page: usize,
    limit: usize,
) -> Result<PaginatedResponse<ApiHadithFamily>> {
    let page = page.max(1);
    let limit = limit.clamp(1, 100);
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
        .context("family list query failed")?;
    let families: Vec<HadithFamily> = res.take(0).unwrap_or_default();
    let has_more = families.len() > limit;
    Ok(PaginatedResponse {
        data: families
            .into_iter()
            .take(limit)
            .map(ApiHadithFamily::from)
            .collect(),
        page,
        limit,
        has_more,
        total: None,
    })
}

/// Family detail — the family record + all hadith variants in `hadith_number`
/// order. `Err` with a not-found marker when the id is missing.
pub async fn get_detail(state: &AppState, id: &str) -> Result<ApiFamilyDetail> {
    let fid = make_record_id("hadith_family", id);
    let mut res = state
        .db
        .query(format!(
            "SELECT * FROM $fid; \
             SELECT {HADITH_FIELDS} FROM hadith WHERE family_id = $fid ORDER BY hadith_number ASC;"
        ))
        .bind(("fid", fid))
        .await
        .context("family detail query failed")?;
    let family: HadithFamily = res
        .take::<Option<HadithFamily>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("family not found: {id}"))?;
    let hadiths: Vec<Hadith> = res.take(1).unwrap_or_default();
    Ok(ApiFamilyDetail {
        family: ApiHadithFamily::from(family),
        hadiths: hadiths.into_iter().map(ApiHadith::from).collect(),
    })
}

/// Distinguishes "family not found" so HTTP returns 404 / MCP returns
/// `invalid_request` instead of 500.
pub fn is_not_found(e: &anyhow::Error) -> bool {
    e.to_string().starts_with("family not found:")
}

/// Aggregate counts across `hadith_family` and `isnad_analysis`.
pub async fn mustalah_stats(state: &AppState) -> Result<ApiMustalahStats> {
    #[derive(Debug, SurrealValue)]
    struct CountRow {
        c: i64,
    }
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
        .context("mustalah stats query failed")?;

    let take = |row: Option<CountRow>| -> i64 { row.map(|c| c.c).unwrap_or(0) };
    Ok(ApiMustalahStats {
        family_count: take(res.take(0).unwrap_or(None)),
        analyzed_count: take(res.take(1).unwrap_or(None)),
        mutawatir_count: take(res.take(2).unwrap_or(None)),
        mashhur_count: take(res.take(3).unwrap_or(None)),
        aziz_count: take(res.take(4).unwrap_or(None)),
        gharib_count: take(res.take(5).unwrap_or(None)),
    })
}

/// Full mustalah analysis bundle for one family (analysis + chains + pivots).
/// Returns `Ok` even when the family has no analysis row (every section
/// independently optional / possibly empty).
pub async fn mustalah_family_analysis(
    state: &AppState,
    id: &str,
) -> Result<ApiMustalahFamilyAnalysis> {
    let fid = make_record_id("hadith_family", id);

    #[derive(Debug, SurrealValue)]
    struct ChainRowDb {
        variant: Option<RecordId>,
        narrator_count: Option<i64>,
        has_chronology_conflict: Option<bool>,
        narrator_ids: Option<Vec<String>>,
    }
    #[derive(Debug, SurrealValue)]
    struct PivotRowDb {
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
        .context("mustalah family analysis query failed")?;

    let analysis: Option<MustalahAnalysisRow> = res.take(0).unwrap_or(None);
    let chains_db: Vec<ChainRowDb> = res.take(1).unwrap_or_default();
    let pivots_db: Vec<PivotRowDb> = res.take(2).unwrap_or_default();

    let chains = chains_db
        .into_iter()
        .map(|c| MustalahChainRow {
            variant_id: c
                .variant
                .as_ref()
                .map(record_id_key_string)
                .unwrap_or_default(),
            narrator_count: c.narrator_count,
            has_chronology_conflict: c.has_chronology_conflict,
            narrator_ids: c.narrator_ids,
        })
        .collect();
    let pivots = pivots_db
        .into_iter()
        .map(|p| MustalahPivotRow {
            narrator_id: p
                .narrator
                .as_ref()
                .map(record_id_key_string)
                .unwrap_or_default(),
            bundle_coverage: p.bundle_coverage,
            fan_out: p.fan_out,
            collector_diversity: p.collector_diversity,
            bypass_count: p.bypass_count,
            is_bottleneck: p.is_bottleneck,
        })
        .collect();

    Ok(ApiMustalahFamilyAnalysis {
        analysis,
        chains,
        pivots,
    })
}
