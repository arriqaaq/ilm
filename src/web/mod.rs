pub mod book_handlers;
pub mod handlers;
pub mod note_handlers;
pub mod quran_handlers;
pub mod sse;

use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use surrealdb::Surreal;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::book_chat::{BookTree, NavCache};
use crate::db::Db;
use crate::embed::{RerankBackendKind, RerankerBackend};
use crate::embedding::{EmbedConfig, EmbeddingProvider, build_embedder};
use crate::llm::{LlmConfig, LlmProvider, build_provider};

#[derive(Clone)]
pub struct AppState {
    pub db: Surreal<Db>,
    pub embedder: Option<Arc<dyn EmbeddingProvider>>,
    pub reranker: Option<Arc<RerankerBackend>>,
    pub llm: Option<Arc<dyn LlmProvider>>,
    pub book_trees: Option<Arc<std::collections::HashMap<u64, BookTree>>>,
    pub nav_cache: Arc<NavCache>,
    pub advanced_enabled: bool,
}

pub struct ServeConfig {
    pub port: u16,
    /// `None` = lite mode, no LLM provider (Ask/classification/LLM-rerank disabled).
    pub llm: Option<LlmConfig>,
    /// `None` = lite mode, no embedder (text-only search; no semantic/hybrid).
    pub embed: Option<EmbedConfig>,
    pub rerank_backend: Option<RerankBackendKind>,
    pub reranker_model: Option<String>,
    pub pageindex_dir: Option<String>,
}

pub async fn serve(db: Surreal<Db>, cfg: ServeConfig) -> Result<()> {
    let advanced_enabled = cfg!(feature = "advanced");

    let embedder = match cfg.embed {
        Some(ref ecfg)
            if advanced_enabled
                || ecfg.provider != crate::embedding::EmbedProviderKind::Fastembed =>
        {
            tracing::info!(
                "Embeddings: provider={} model={}",
                ecfg.provider.as_str(),
                ecfg.model
            );
            Some(build_embedder(ecfg)?)
        }
        Some(_) => {
            tracing::info!("Advanced features disabled — skipping fastembed embedder");
            None
        }
        None => {
            tracing::info!("No --embed-model — running in text-only (lite) mode");
            None
        }
    };

    let llm_client: Option<Arc<dyn LlmProvider>> = match cfg.llm {
        Some(ref lcfg) => {
            tracing::info!(
                "LLM: provider={} model={}",
                lcfg.provider.as_str(),
                lcfg.model
            );
            Some(build_provider(lcfg)?)
        }
        None => {
            tracing::info!("No --llm-model — Ask, classification, and LLM rerank disabled");
            None
        }
    };

    let reranker = match cfg.rerank_backend {
        #[cfg(feature = "advanced")]
        Some(RerankBackendKind::Fastembed) => {
            tracing::info!("Loading fastembed reranker: bge-reranker-v2-m3");
            Some(Arc::new(RerankerBackend::Fastembed(
                crate::embed::FastembedReranker::new()?,
            )))
        }
        Some(RerankBackendKind::Llm) => match llm_client.as_ref() {
            Some(provider) => {
                tracing::info!(
                    "Using LLM reranker via {} (model: {})",
                    provider.provider_name(),
                    cfg.reranker_model
                        .as_deref()
                        .unwrap_or(provider.default_model())
                );
                Some(Arc::new(RerankerBackend::Llm {
                    provider: Arc::clone(provider),
                    model: cfg.reranker_model,
                }))
            }
            None => {
                tracing::warn!("--reranker llm requires an --llm-model; reranker disabled");
                None
            }
        },
        #[cfg(not(feature = "advanced"))]
        Some(RerankBackendKind::Fastembed) => {
            tracing::warn!("Fastembed reranker requires advanced features — ignoring");
            None
        }
        None => None,
    };

    let llm = llm_client;

    let book_trees = if let Some(dir) = cfg.pageindex_dir {
        let path = std::path::Path::new(&dir);
        match crate::book_chat::load_book_trees(path) {
            Ok(trees) => {
                tracing::info!("Loaded {} books from PageIndex", trees.len());
                Some(Arc::new(trees))
            }
            Err(e) => {
                tracing::warn!("Failed to load PageIndex book trees: {e}");
                None
            }
        }
    } else {
        None
    };

    let state = AppState {
        db,
        embedder,
        reranker,
        llm,
        book_trees,
        nav_cache: Arc::new(NavCache::new()),
        advanced_enabled,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .route("/api/config", axum::routing::get(handlers::app_config))
        .route("/api/stats", axum::routing::get(handlers::stats))
        .route("/api/collections", axum::routing::get(handlers::books))
        .route("/api/search", axum::routing::get(handlers::search))
        .route("/api/hadiths", axum::routing::get(handlers::hadith_list))
        .route(
            "/api/hadiths/{id}",
            axum::routing::get(handlers::hadith_detail),
        )
        .route(
            "/api/narrators",
            axum::routing::get(handlers::narrator_list),
        )
        .route(
            "/api/narrators/{id}",
            axum::routing::get(handlers::narrator_detail).put(handlers::update_narrator),
        )
        .route(
            "/api/narrators/autocomplete",
            axum::routing::get(handlers::narrator_autocomplete),
        )
        .route(
            "/api/narrators/common",
            axum::routing::get(handlers::common_narrators),
        )
        .route(
            "/api/isnad/search",
            axum::routing::post(handlers::isnad_search),
        )
        .route(
            "/api/chain/{hadith_id}",
            axum::routing::get(handlers::chain_graph_data),
        )
        .route(
            "/api/narrators/{id}/graph",
            axum::routing::get(handlers::narrator_graph_data),
        )
        .route("/api/ask", axum::routing::post(handlers::ask))
        .route("/api/families", axum::routing::get(handlers::family_list))
        .route(
            "/api/families/{id}",
            axum::routing::get(handlers::family_detail),
        )
        .route(
            "/api/analysis/stats",
            axum::routing::get(handlers::mustalah_stats),
        )
        .route(
            "/api/hadiths/{id}/gradings",
            axum::routing::get(handlers::hadith_gradings),
        )
        .route(
            "/api/families/{id}/mustalah",
            axum::routing::get(handlers::mustalah_family_analysis),
        )
        .route(
            "/api/narrators/{id}/isnad-role",
            axum::routing::get(handlers::narrator_isnad_role),
        )
        .route("/api/diff", axum::routing::get(handlers::matn_diff_handler))
        .route(
            "/api/export/family/{id}",
            axum::routing::get(handlers::export_family),
        )
        .route(
            "/api/internal/translate",
            axum::routing::post(handlers::update_translation),
        )
        // Quran routes
        .route(
            "/api/quran/stats",
            axum::routing::get(quran_handlers::quran_stats),
        )
        .route(
            "/api/quran/surahs",
            axum::routing::get(quran_handlers::surah_list),
        )
        .route(
            "/api/quran/surahs/{number}",
            axum::routing::get(quran_handlers::surah_detail),
        )
        .route(
            "/api/quran/search",
            axum::routing::get(quran_handlers::quran_search),
        )
        .route(
            "/api/quran/browse",
            axum::routing::get(quran_handlers::ayah_browse),
        )
        .route(
            "/api/quran/ask",
            axum::routing::post(quran_handlers::ask_quran),
        )
        .route(
            "/api/quran/ayah/{ayah_key}/hadiths",
            axum::routing::get(quran_handlers::ayah_hadiths),
        )
        .route(
            "/api/quran/surahs/{number}/hadith-counts",
            axum::routing::get(quran_handlers::surah_hadith_counts),
        )
        .route(
            "/api/quran/surahs/{number}/similar-counts",
            axum::routing::get(quran_handlers::surah_similar_counts),
        )
        .route(
            "/api/quran/ayah/{ayah_key}/words",
            axum::routing::get(quran_handlers::ayah_words),
        )
        .route(
            "/api/quran/search/root/{root}",
            axum::routing::get(quran_handlers::root_search),
        )
        .route(
            "/api/quran/reciters",
            axum::routing::get(quran_handlers::reciters),
        )
        .route(
            "/api/quran/ayah/{ayah_key}/similar",
            axum::routing::get(quran_handlers::ayah_similar),
        )
        .route(
            "/api/quran/phrases/{id}",
            axum::routing::get(quran_handlers::phrase_detail),
        )
        // Book viewer routes
        .route(
            "/api/books/config",
            axum::routing::get(book_handlers::books_config),
        )
        .route(
            "/api/books/list",
            axum::routing::get(book_handlers::list_books),
        )
        .route(
            "/api/books/{book_id}",
            axum::routing::get(book_handlers::get_book),
        )
        .route(
            "/api/books/{book_id}/pages",
            axum::routing::get(book_handlers::get_pages),
        )
        .route(
            "/api/quran/surah/{number}/tafsir-pages",
            axum::routing::get(book_handlers::surah_tafsir_pages),
        )
        .route(
            "/api/quran/ayah/{surah}/{ayah}/tafsir",
            axum::routing::get(book_handlers::ayah_tafsir),
        )
        .route(
            "/api/tafsir/ayah/{surah}/{ayah}/all",
            axum::routing::get(book_handlers::ayah_tafsirs_all),
        )
        .route(
            "/api/tafsir/ask",
            axum::routing::post(book_handlers::tafsir_ask),
        )
        .route(
            "/api/hadiths/sharh-pages",
            axum::routing::get(book_handlers::hadith_sharh_pages),
        )
        .route(
            "/api/narrators/{id}/books",
            axum::routing::get(book_handlers::narrator_books),
        )
        .route(
            "/api/books/{book_id}/chat",
            axum::routing::post(book_handlers::book_chat),
        )
        // Unified Quran & Sunnah routes
        .route(
            "/api/unified/search",
            axum::routing::get(handlers::unified_search),
        )
        .route(
            "/api/unified/ask",
            axum::routing::post(handlers::unified_ask),
        )
        // Link preview
        .route(
            "/api/link-preview",
            axum::routing::get(handlers::link_preview),
        )
        // Notes — specific paths BEFORE {id} to avoid Axum matching
        .route(
            "/api/notes",
            axum::routing::get(note_handlers::list_notes).post(note_handlers::create_note),
        )
        .route(
            "/api/notes/refs",
            axum::routing::get(note_handlers::bulk_note_refs),
        )
        .route(
            "/api/notes/tags",
            axum::routing::get(note_handlers::list_tags),
        )
        .route(
            "/api/notes/export",
            axum::routing::get(note_handlers::export_notes),
        )
        .route(
            "/api/notes/{id}",
            axum::routing::get(note_handlers::get_note)
                .put(note_handlers::update_note)
                .delete(note_handlers::delete_note),
        )
        .route(
            "/api/notes/{id}/refs",
            axum::routing::put(note_handlers::update_note_refs),
        )
        .route(
            "/api/notes/{id}/refs/{idx}/annotation",
            axum::routing::put(note_handlers::update_ref_annotation),
        )
        // Notebooks
        .route(
            "/api/notebooks",
            axum::routing::get(note_handlers::list_notebooks).post(note_handlers::create_notebook),
        )
        .route(
            "/api/notebooks/{id}",
            axum::routing::put(note_handlers::update_notebook)
                .delete(note_handlers::delete_notebook),
        )
        .with_state(state);

    // Serve static assets from frontend/build, with SPA fallback to index.html
    let spa_fallback = ServeFile::new("frontend/build/index.html");
    let static_files = ServeDir::new("frontend/build").not_found_service(spa_fallback);

    let app = api.fallback_service(static_files).layer(cors);

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("Server listening on http://localhost:{}", cfg.port);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
