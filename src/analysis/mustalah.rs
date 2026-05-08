//! Structural analysis of hadith family transmission chains.
//!
//! For each family this engine produces:
//! - per chain: ordered narrator list, narrator count, chronology-conflict flag
//! - breadth: narrator count per tabaqah, plus a class
//!   (mutawatir / mashhur / aziz / gharib) computed from the minimum
//! - pivots: narrators with high bundle coverage / fan-out (madar candidates)
//! - defect flags: chronology conflicts
//!
//! No grading. Narrator-level assessments come from scholarly sources stored
//! in the `evidence` table.

use std::cmp::Ordering;

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::RecordId;

use super::isnad_graph::{self, Direction, FamilyGraph};
use crate::db::Db;

// ── Enums ──

/// Transmission breadth classification.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BreadthClass {
    Mutawatir,
    Mashhur,
    Aziz,
    Gharib,
}

// ── Result structs ──

/// Structural assessment of a single chain (variant).
#[derive(Debug, Clone, Serialize)]
pub struct ChainAssessment {
    pub variant_id: String,
    pub narrator_count: usize,
    pub has_chronology_conflict: bool,
    /// Narrator IDs in chain order (student → teacher → ... → source).
    pub narrator_ids: Vec<String>,
}

/// Transmission breadth analysis.
#[derive(Debug, Clone, Serialize)]
pub struct TransmissionBreadth {
    pub classification: BreadthClass,
    /// (tabaqah_number, narrator_count) pairs in ascending tabaqah order.
    pub breadth_per_tabaqah: Vec<(i64, usize)>,
    pub min_breadth: usize,
    pub bottleneck_tabaqah: Option<i64>,
    pub fard_narrator: Option<String>,
}

/// Pivot narrator (madar al-isnad) info.
#[derive(Debug, Clone, Serialize)]
pub struct PivotNarrator {
    pub narrator_id: String,
    pub bundle_coverage: f64,
    pub fan_out: usize,
    pub collector_diversity: usize,
    pub bypass_count: usize,
    pub is_bottleneck: bool,
}

/// Detected defect flags.
#[derive(Debug, Clone, Serialize)]
pub struct DefectFlags {
    pub has_chronology_conflict: bool,
    pub flags: Vec<String>,
}

/// Complete structural analysis result for one hadith family.
#[derive(Debug, Clone, Serialize)]
pub struct FamilyMustalahResult {
    pub family_id: String,
    pub chains: Vec<ChainAssessment>,
    pub breadth: TransmissionBreadth,
    pub pivots: Vec<PivotNarrator>,
    pub defects: DefectFlags,
}

// ══════════════════════════════════════════════════════════
// 1. Per-chain structural assessment
// ══════════════════════════════════════════════════════════

fn assess_chain(graph: &FamilyGraph, variant_id: &str) -> ChainAssessment {
    let chain = graph.chain_for_variant(variant_id);
    let narrator_count = chain.len();

    let mut has_chronology_conflict = false;
    for i in 0..chain.len().saturating_sub(1) {
        let edge_key = format!("{}->{}", chain[i], chain[i + 1]);
        if let Some(edges) = graph.edges.get(&edge_key) {
            for e in edges {
                if e.chronology_conflict {
                    has_chronology_conflict = true;
                }
            }
        }
    }

    ChainAssessment {
        variant_id: variant_id.to_string(),
        narrator_count,
        has_chronology_conflict,
        narrator_ids: chain,
    }
}

// ══════════════════════════════════════════════════════════
// 2. Transmission breadth
// ══════════════════════════════════════════════════════════

fn compute_breadth(graph: &FamilyGraph) -> TransmissionBreadth {
    let tabaqat = graph.tabaqat();
    if tabaqat.is_empty() {
        return TransmissionBreadth {
            classification: BreadthClass::Gharib,
            breadth_per_tabaqah: vec![],
            min_breadth: 0,
            bottleneck_tabaqah: None,
            fard_narrator: None,
        };
    }

    let mut breadth_per_tabaqah: Vec<(i64, usize)> = Vec::new();
    let mut min_breadth = usize::MAX;
    let mut bottleneck_tabaqah = None;
    let mut fard_narrator = None;

    for &tab in &tabaqat {
        let mut narrators: Vec<&str> = graph.narrators_at_tabaqah(tab);
        narrators.sort();
        let count = narrators.len();
        breadth_per_tabaqah.push((tab, count));
        if count < min_breadth {
            min_breadth = count;
            bottleneck_tabaqah = Some(tab);
            fard_narrator = if count == 1 {
                narrators.first().map(|s| s.to_string())
            } else {
                None
            };
        }
    }

    if min_breadth == usize::MAX {
        min_breadth = 0;
    }

    let classification = if min_breadth > 3 {
        BreadthClass::Mutawatir
    } else if min_breadth == 3 {
        BreadthClass::Mashhur
    } else if min_breadth == 2 {
        BreadthClass::Aziz
    } else {
        BreadthClass::Gharib
    };

    TransmissionBreadth {
        classification,
        breadth_per_tabaqah,
        min_breadth,
        bottleneck_tabaqah,
        fard_narrator,
    }
}

// ══════════════════════════════════════════════════════════
// 3. Pivot narrators (madar al-isnad)
// ══════════════════════════════════════════════════════════

fn identify_pivots(graph: &mut FamilyGraph) -> Vec<PivotNarrator> {
    let total_variants = graph.variant_ids.len();
    if total_variants == 0 {
        return vec![];
    }

    graph.ensure_variant_narrator_map();

    // Sort upfront so iteration order is deterministic.
    let mut nids: Vec<String> = graph.nodes.keys().cloned().collect();
    nids.sort();
    let mut pivots: Vec<PivotNarrator> = Vec::new();

    for nid in &nids {
        let node = &graph.nodes[nid];
        let fan_out = node.direct_students.len();
        let bundle_coverage = node.variants.len() as f64 / total_variants as f64;
        let collector_diversity = graph.reachable_terminals(nid).len();
        let bypass_count = {
            let missing: Vec<String> = graph
                .variant_ids
                .iter()
                .filter(|v| !node.variants.contains(*v))
                .cloned()
                .collect();
            let ancestors = graph.reachable_set(nid, Direction::Teachers);
            let descendants = graph.reachable_set(nid, Direction::Students);
            let vmap = graph.variant_narrator_map().unwrap();
            missing
                .iter()
                .filter(|v| {
                    vmap.get(*v).is_some_and(|narrs| {
                        narrs.iter().any(|n| ancestors.contains(n))
                            && narrs.iter().any(|n| descendants.contains(n))
                    })
                })
                .count()
        };

        if bundle_coverage >= 0.20 || fan_out >= 2 {
            let is_bottleneck = bundle_coverage >= 0.95;
            pivots.push(PivotNarrator {
                narrator_id: nid.clone(),
                bundle_coverage,
                fan_out,
                collector_diversity,
                bypass_count,
                is_bottleneck,
            });
        }
    }

    pivots.sort_by(|a, b| {
        b.bundle_coverage
            .partial_cmp(&a.bundle_coverage)
            .unwrap_or(Ordering::Equal)
            .then(b.fan_out.cmp(&a.fan_out))
            .then(a.narrator_id.cmp(&b.narrator_id))
    });

    pivots.truncate(10);
    pivots
}

// ══════════════════════════════════════════════════════════
// 4. Defect detection
// ══════════════════════════════════════════════════════════

fn detect_defects(_graph: &FamilyGraph, chains: &[ChainAssessment]) -> DefectFlags {
    let mut flags: Vec<String> = Vec::new();
    let has_chronology_conflict = chains.iter().any(|c| c.has_chronology_conflict);

    if has_chronology_conflict {
        flags.push("Chronology conflict detected: student's generation predates teacher's".into());
    }

    DefectFlags {
        has_chronology_conflict,
        flags,
    }
}

// ══════════════════════════════════════════════════════════
// Main orchestrator
// ══════════════════════════════════════════════════════════

pub async fn analyze_family_mustalah(
    db: &Surreal<Db>,
    family_id: &str,
) -> Result<Option<FamilyMustalahResult>> {
    let mut graph = match isnad_graph::build_family_graph(db, family_id).await? {
        Some(g) => g,
        None => return Ok(None),
    };

    graph.ensure_variant_narrator_map();

    // Sort variant order so per-chain slug `chain_<family>_<i>` is stable across runs.
    let mut variant_ids: Vec<String> = graph.variant_ids.iter().cloned().collect();
    variant_ids.sort();

    let chains: Vec<ChainAssessment> = variant_ids
        .iter()
        .map(|vid| assess_chain(&graph, vid))
        .collect();

    let breadth = compute_breadth(&graph);
    let pivots = identify_pivots(&mut graph);
    let defects = detect_defects(&graph, &chains);

    Ok(Some(FamilyMustalahResult {
        family_id: family_id.to_string(),
        chains,
        breadth,
        pivots,
        defects,
    }))
}

pub async fn store_mustalah_results(db: &Surreal<Db>, result: &FamilyMustalahResult) -> Result<()> {
    let family_rid = RecordId::new("hadith_family", result.family_id.as_str());
    let slug = format!("isnad_{}", result.family_id);

    db.query(
        "CREATE $rid CONTENT { \
            family: $family, \
            breadth_class: $breadth_class, \
            min_breadth: $min_breadth, \
            bottleneck_tabaqah: $bottleneck_tabaqah, \
            chain_count: $chain_count, \
            ilal_flags: $ilal_flags \
        }",
    )
    .bind(("rid", RecordId::new("isnad_analysis", slug.as_str())))
    .bind(("family", family_rid.clone()))
    .bind((
        "breadth_class",
        format!("{:?}", result.breadth.classification).to_lowercase(),
    ))
    .bind(("min_breadth", result.breadth.min_breadth as i64))
    .bind(("bottleneck_tabaqah", result.breadth.bottleneck_tabaqah))
    .bind(("chain_count", result.chains.len() as i64))
    .bind(("ilal_flags", result.defects.flags.clone()))
    .await?;

    for (i, chain) in result.chains.iter().enumerate() {
        let chain_slug = format!("chain_{}_{}", result.family_id, i);
        db.query(
            "CREATE $rid CONTENT { \
                family: $family, \
                variant: $variant, \
                narrator_count: $narrator_count, \
                has_chronology_conflict: $chrono, \
                narrator_ids: $narrator_ids \
            }",
        )
        .bind((
            "rid",
            RecordId::new("chain_assessment", chain_slug.as_str()),
        ))
        .bind(("family", family_rid.clone()))
        .bind((
            "variant",
            RecordId::new("hadith", chain.variant_id.as_str()),
        ))
        .bind(("narrator_count", chain.narrator_count as i64))
        .bind(("chrono", chain.has_chronology_conflict))
        .bind(("narrator_ids", chain.narrator_ids.clone()))
        .await?;
    }

    for pivot in &result.pivots {
        let pivot_slug = format!("pivot_{}_{}", result.family_id, pivot.narrator_id);
        db.query(
            "CREATE $rid CONTENT { \
                family: $family, \
                narrator: $narrator, \
                bundle_coverage: $coverage, \
                fan_out: $fan_out, \
                collector_diversity: $diversity, \
                bypass_count: $bypass, \
                is_bottleneck: $bottleneck \
            }",
        )
        .bind(("rid", RecordId::new("narrator_pivot", pivot_slug.as_str())))
        .bind(("family", family_rid.clone()))
        .bind((
            "narrator",
            RecordId::new("narrator", pivot.narrator_id.as_str()),
        ))
        .bind(("coverage", pivot.bundle_coverage))
        .bind(("fan_out", pivot.fan_out as i64))
        .bind(("diversity", pivot.collector_diversity as i64))
        .bind(("bypass", pivot.bypass_count as i64))
        .bind(("bottleneck", pivot.is_bottleneck))
        .await?;
    }

    Ok(())
}
