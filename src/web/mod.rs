pub mod book_handlers;
pub mod docs;
pub mod handlers;
pub mod note_handlers;
pub mod openapi;
pub mod quran_handlers;
pub mod sse;

use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::response::Json;
use surrealdb::Surreal;
use tokio::net::TcpListener;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_scalar::{Scalar, Servable};

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
    /// Lite mode: when `None`, no LLM is wired up (no Ask, no classification,
    /// no LLM rerank).
    pub llm: Option<LlmConfig>,
    /// Lite mode: when `None`, no embedder is wired up (text-only search).
    pub embed: Option<EmbedConfig>,
    pub rerank_backend: Option<RerankBackendKind>,
    pub reranker_model: Option<String>,
    pub pageindex_dir: Option<String>,
}

pub async fn serve(db: Surreal<Db>, cfg: ServeConfig) -> Result<()> {
    let advanced_enabled = cfg!(feature = "advanced");

    let embedder = match cfg.embed.as_ref() {
        Some(embed_cfg)
            if advanced_enabled
                || embed_cfg.provider != crate::embedding::EmbedProviderKind::Fastembed =>
        {
            Some(build_embedder(embed_cfg)?)
        }
        Some(_) => {
            tracing::info!("Advanced features disabled — skipping embedding model");
            None
        }
        None => {
            tracing::info!("Lite mode — no embedder configured");
            None
        }
    };

    let llm_client = match cfg.llm.as_ref() {
        Some(llm_cfg) => Some(Arc::clone(&build_provider(llm_cfg)?)),
        None => {
            tracing::info!("Lite mode — no LLM configured (Ask endpoints will return 503)");
            None
        }
    };

    let reranker = match (cfg.rerank_backend, llm_client.as_ref()) {
        #[cfg(feature = "advanced")]
        (Some(RerankBackendKind::Fastembed), _) => {
            tracing::info!("Loading fastembed reranker: bge-reranker-v2-m3");
            Some(Arc::new(RerankerBackend::Fastembed(
                crate::embed::FastembedReranker::new()?,
            )))
        }
        (Some(RerankBackendKind::Llm), Some(llm)) => {
            tracing::info!(
                "Using LLM reranker via {} (model: {})",
                llm.provider_name(),
                cfg.reranker_model.as_deref().unwrap_or(llm.default_model())
            );
            Some(Arc::new(RerankerBackend::Llm {
                provider: Arc::clone(llm),
                model: cfg.reranker_model,
            }))
        }
        (Some(RerankBackendKind::Llm), None) => {
            tracing::warn!("LLM reranker requested but no LLM configured — ignoring");
            None
        }
        #[cfg(not(feature = "advanced"))]
        (Some(RerankBackendKind::Fastembed), _) => {
            tracing::warn!("Fastembed reranker requires advanced features — ignoring");
            None
        }
        (None, _) => None,
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

    // ── Public v1 API: OpenApiRouter so handlers register themselves into the spec ──
    //
    // Documented endpoints. Read-only; rate-limited per IP. The legacy `/api/*`
    // surface is gone — all read endpoints have moved here. Internal/admin
    // endpoints live under `/internal/*` and do NOT appear in the OpenAPI spec.
    let v1_read_router: OpenApiRouter<AppState> = OpenApiRouter::new()
        // Meta
        .routes(routes!(handlers::app_config))
        .routes(routes!(handlers::stats))
        // Hadith
        .routes(routes!(handlers::books))
        .routes(routes!(handlers::hadith_list))
        .routes(routes!(handlers::hadith_detail))
        .routes(routes!(handlers::chain_graph_data))
        .routes(routes!(handlers::hadith_gradings))
        .routes(routes!(handlers::list_scholars))
        .routes(routes!(handlers::matn_diff_handler))
        .routes(routes!(book_handlers::hadith_sharh_pages))
        // Narrators
        .routes(routes!(handlers::narrator_list))
        .routes(routes!(handlers::narrator_autocomplete))
        .routes(routes!(handlers::common_narrators))
        .routes(routes!(handlers::narrator_detail))
        .routes(routes!(handlers::narrator_graph_data))
        .routes(routes!(handlers::narrator_isnad_role))
        .routes(routes!(book_handlers::narrator_books))
        // Isnad / Families / Mustalah
        .routes(routes!(handlers::isnad_search))
        .routes(routes!(handlers::family_list))
        .routes(routes!(handlers::family_detail))
        .routes(routes!(handlers::mustalah_family_analysis))
        .routes(routes!(handlers::export_family))
        .routes(routes!(handlers::mustalah_stats))
        // Quran
        .routes(routes!(quran_handlers::quran_stats))
        .routes(routes!(quran_handlers::surah_list))
        .routes(routes!(quran_handlers::surah_detail))
        .routes(routes!(quran_handlers::ayah_browse))
        .routes(routes!(quran_handlers::ayah_words))
        .routes(routes!(quran_handlers::ayah_hadiths))
        .routes(routes!(quran_handlers::surah_hadith_counts))
        .routes(routes!(quran_handlers::surah_similar_counts))
        .routes(routes!(quran_handlers::ayah_similar))
        .routes(routes!(quran_handlers::phrase_detail))
        .routes(routes!(quran_handlers::root_search))
        .routes(routes!(quran_handlers::reciters))
        .routes(routes!(book_handlers::surah_tafsir_pages))
        .routes(routes!(book_handlers::ayah_tafsir))
        .routes(routes!(book_handlers::ayah_tafsirs_all))
        // Books / Tafsir
        .routes(routes!(book_handlers::books_config))
        .routes(routes!(book_handlers::list_books))
        .routes(routes!(book_handlers::get_book))
        .routes(routes!(book_handlers::get_pages))
        // Search
        .routes(routes!(handlers::search))
        .routes(routes!(quran_handlers::quran_search))
        .routes(routes!(handlers::unified_search));

    // ── Ask sub-router: same OpenAPI surface but stricter rate limit ──
    let v1_ask_router: OpenApiRouter<AppState> = OpenApiRouter::new()
        .routes(routes!(handlers::ask))
        .routes(routes!(handlers::unified_ask))
        .routes(routes!(quran_handlers::ask_quran))
        .routes(routes!(book_handlers::tafsir_ask))
        .routes(routes!(book_handlers::book_chat));

    // Per-IP rate limit: 60 req/min default for read endpoints; 10 req/min for ask endpoints.
    // tower_governor uses `per_second` + `burst_size`. burst_size sets the
    // quota refill bucket size; per_second sets the steady-state rate.
    // 60/min ≈ 1 req/sec with burst of 30 → tolerates short spikes.
    let read_governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(30)
            .finish()
            .expect("valid governor config"),
    );
    let ask_governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(6) // 1 req per 6s ≈ 10/min
            .burst_size(3)
            .finish()
            .expect("valid governor config"),
    );

    // Combine read + ask routers, take the OpenApi document, then attach state.
    let (v1_read_axum, mut openapi_doc) =
        v1_read_router.with_state(state.clone()).split_for_parts();
    let (v1_ask_axum, ask_openapi) = v1_ask_router.with_state(state.clone()).split_for_parts();
    openapi_doc.merge(ask_openapi);

    // Inject the metadata-only base document (info/tags/servers/components).
    let base = docs::base_openapi();
    openapi_doc.info = base.info;
    openapi_doc.servers = base.servers;
    openapi_doc.tags = base.tags;
    if let Some(base_components) = base.components {
        openapi_doc.components = match openapi_doc.components.take() {
            Some(mut existing) => {
                existing.schemas.extend(base_components.schemas);
                existing.responses.extend(base_components.responses);
                Some(existing)
            }
            None => Some(base_components),
        };
    }

    // Each handler's `#[utoipa::path(path = "...")]` already declares the full
    // sub-path (`/ask/hadith`, `/books/{book_id}/ask`, etc.) so we just merge —
    // no extra `nest()` needed. The strict rate limit is attached once to the
    // ask router; the `/books/{book_id}/ask` chat endpoint sits inside it for
    // exactly that reason.
    let v1_router = Router::new()
        .merge(v1_read_axum.layer(GovernorLayer::new(read_governor)))
        .merge(v1_ask_axum.layer(GovernorLayer::new(ask_governor)));

    // ── Internal API: not in OpenAPI spec, used only by the SvelteKit frontend ──
    let internal_router = Router::new()
        // Notes
        .route(
            "/notes",
            axum::routing::get(note_handlers::list_notes).post(note_handlers::create_note),
        )
        .route(
            "/notes/refs",
            axum::routing::get(note_handlers::bulk_note_refs),
        )
        .route("/notes/tags", axum::routing::get(note_handlers::list_tags))
        .route(
            "/notes/export",
            axum::routing::get(note_handlers::export_notes),
        )
        .route(
            "/notes/{id}",
            axum::routing::get(note_handlers::get_note)
                .put(note_handlers::update_note)
                .delete(note_handlers::delete_note),
        )
        .route(
            "/notes/{id}/refs",
            axum::routing::put(note_handlers::update_note_refs),
        )
        .route(
            "/notes/{id}/refs/{idx}/annotation",
            axum::routing::put(note_handlers::update_ref_annotation),
        )
        // Notebooks
        .route(
            "/notebooks",
            axum::routing::get(note_handlers::list_notebooks).post(note_handlers::create_notebook),
        )
        .route(
            "/notebooks/{id}",
            axum::routing::put(note_handlers::update_notebook)
                .delete(note_handlers::delete_notebook),
        )
        // Admin writes / utility endpoints — kept private
        .route(
            "/narrators/{id}",
            axum::routing::put(handlers::update_narrator),
        )
        .route(
            "/translate",
            axum::routing::post(handlers::update_translation),
        )
        .route("/link-preview", axum::routing::get(handlers::link_preview))
        .with_state(state);

    // ── Docs surface: /openapi.json + /docs (Scalar) ──
    let openapi_for_json = openapi_doc.clone();
    let openapi_for_scalar = openapi_doc.clone();
    let docs_router = Router::new()
        .route(
            "/openapi.json",
            axum::routing::get(move || async move { Json(openapi_for_json) }),
        )
        .merge(Scalar::with_url("/docs", openapi_for_scalar));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Serve static assets from frontend/build, with SPA fallback to index.html.
    let spa_fallback = ServeFile::new("frontend/build/index.html");
    let static_files = ServeDir::new("frontend/build").not_found_service(spa_fallback);

    let app = Router::new()
        .nest("/v1", v1_router)
        .nest("/internal", internal_router)
        .merge(docs_router)
        .fallback_service(static_files)
        .layer(cors);

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("Server listening on http://localhost:{}", cfg.port);
    tracing::info!(
        "Interactive docs available at http://localhost:{}/docs",
        cfg.port
    );
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
