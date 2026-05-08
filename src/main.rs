#[cfg(feature = "advanced")]
use hadith::analysis;
use hadith::embed::RerankBackendKind;
use hadith::embedding::{EmbedConfig, EmbedProviderKind, FastembedModelKind};
use hadith::llm::{LlmConfig, LlmProviderKind};
use hadith::web::ServeConfig;
use hadith::{db, embed, ingest, quran, web};

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "hadith",
    about = "Hadith Explorer — browse and search Islamic hadith collections"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest hadith data from SemanticHadith KG (6 major books with identified narrators)
    Ingest {
        /// Path to SemanticHadith JSON data file
        #[arg(long, default_value = "data/semantic_hadith.json")]
        file: String,

        /// Max hadiths per book (for testing). Omit to load all.
        #[arg(long)]
        limit: Option<usize>,

        /// Translate hadiths and narrators to English using Ollama
        #[arg(long)]
        translate: bool,

        /// Ollama model for fallback translation (used when --translate is set).
        #[arg(long, default_value = "command-r7b-arabic")]
        translate_model: String,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,

        /// Embedding model: e5-small (faster) or bge-m3 (higher quality).
        /// Data-prep commands always use local fastembed.
        #[arg(long, default_value = "e5-small", value_enum)]
        embed_model: FastembedModelKind,
    },
    /// Run analysis on ingested data (families, narrator enrichment)
    Analyze {
        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,

        /// Compute hadith families from embedding similarity
        #[arg(long)]
        families: bool,

        /// Run mustalah al-hadith analysis on transmission chains
        #[arg(long)]
        mustalah: bool,

        /// Skip families larger than this (degenerate clusters hang analysis)
        #[arg(long, default_value = "500")]
        max_family_size: usize,

        /// Embedding model: e5-small (faster) or bge-m3 (higher quality).
        #[arg(long, default_value = "e5-small", value_enum)]
        embed_model: FastembedModelKind,
    },
    /// Ingest Quran data (Arabic + English + Tafsir Ibn Kathir)
    IngestQuran {
        /// Path to Quran CSV file (from scripts/prepare_quran_data.py)
        #[arg(long, default_value = "data/quran.csv")]
        file: String,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,

        /// Embedding model: e5-small (faster) or bge-m3 (higher quality).
        #[arg(long, default_value = "e5-small", value_enum)]
        embed_model: FastembedModelKind,
    },
    /// Ingest Quran→Hadith reference mappings from Quran.com
    IngestQuranHadithRefs {
        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Ingest word morphology data (corpus.quran.com + QUL translations)
    IngestMorphology {
        /// Path to quran-morphology.txt
        #[arg(long, default_value = "data/quran-morphology.txt")]
        file: String,

        /// Directory with QUL JSON files (colored-english-wbw-translation.json, etc.)
        #[arg(long, default_value = "qul")]
        qul_dir: String,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Ingest shared phrases (mutashabihat) and similar ayahs from QUL JSON
    IngestQuranSimilar {
        /// Directory with QUL JSON files (phrases.json, matching-ayah.json)
        #[arg(long, default_value = "qul")]
        qul_dir: String,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Ingest a turath book (pages + headings) from local JSON files
    IngestTurath {
        /// Path to pages JSON file
        #[arg(long)]
        pages_file: String,

        /// Path to headings JSON file
        #[arg(long)]
        headings_file: String,

        /// Turath book ID
        #[arg(long)]
        book_id: u32,

        /// Book name in Arabic
        #[arg(long)]
        name_ar: String,

        /// Book name in English
        #[arg(long)]
        name_en: String,

        /// Author name in Arabic
        #[arg(long)]
        author_ar: String,

        /// Optional: tafsir ayah mapping file (for Tafsir books)
        #[arg(long)]
        tafsir_mapping: Option<String>,

        /// Optional: hadith sharh mapping file (for Sharh books)
        #[arg(long)]
        sharh_mapping: Option<String>,

        /// Collection book_id for sharh mapping (e.g. 1 for Bukhari)
        #[arg(long)]
        sharh_collection_id: Option<u32>,

        /// Optional: narrator→book mapping file (for narrator bio books)
        #[arg(long)]
        narrator_mapping: Option<String>,

        /// Book category: quran, hadith, narrator
        #[arg(long)]
        category: Option<String>,

        /// Book type: tafsir, sharh, collection, biography
        #[arg(long)]
        book_type: Option<String>,

        /// Force re-ingestion (delete existing data for this book first)
        #[arg(long)]
        force: bool,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Start the web server
    Serve {
        /// Port to listen on
        #[arg(long, default_value_t = 3000)]
        port: u16,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,

        // ── LLM provider ──────────────────────────────────────────────
        // Both --llm-model and --embed-model are OPTIONAL. Omit them for
        // "lite" mode: text-only search, no Ask, no embeddings, no LLM.
        // Useful for browsing data without setting up Ollama/OpenAI/etc.
        /// LLM provider for chat/classification/reranking.
        /// Ignored unless --llm-model is also set.
        #[arg(long, env = "LLM_PROVIDER", default_value = "ollama", value_enum)]
        llm_provider: LlmProviderKind,

        /// LLM model name. Provider-specific (e.g. `llama3.2`, `gpt-4o-mini`,
        /// `claude-opus-4-7`). Omit to disable Ask / classification / LLM rerank.
        #[arg(long, env = "LLM_MODEL")]
        llm_model: Option<String>,

        /// LLM base URL — Ollama only. Defaults to http://localhost:11434.
        #[arg(long, env = "LLM_BASE_URL")]
        llm_base_url: Option<String>,

        /// LLM API key — required for openai/anthropic.
        #[arg(long, env = "LLM_API_KEY")]
        llm_api_key: Option<String>,

        // ── Embedding provider ────────────────────────────────────────
        /// Embedding provider. Ignored unless --embed-model is also set.
        #[arg(long, env = "EMBED_PROVIDER", default_value = "fastembed", value_enum)]
        embed_provider: EmbedProviderKind,

        /// Embedding model name. fastembed: `bge-m3`/`e5-small`. openai:
        /// `text-embedding-3-small`/`-large`. ollama: any embedding model.
        /// Omit to disable embeddings (text-only search; no semantic/hybrid).
        #[arg(long, env = "EMBED_MODEL")]
        embed_model: Option<String>,

        /// Embedding base URL — Ollama only.
        #[arg(long, env = "EMBED_BASE_URL")]
        embed_base_url: Option<String>,

        /// Embedding API key — OpenAI only.
        #[arg(long, env = "EMBED_API_KEY")]
        embed_api_key: Option<String>,

        /// Explicit embedding dimension. Required for ollama, optional for
        /// openai (text-embedding-3-* support reduced dims). Ignored for fastembed.
        #[arg(long, env = "EMBED_DIMENSIONS")]
        embed_dimensions: Option<usize>,

        // ── Reranker ──────────────────────────────────────────────────
        /// Reranker backend (opt-in via `rerank=true` query param on hybrid search).
        /// Omit to disable reranking.
        /// - fastembed: local cross-encoder (BAAI/bge-reranker-v2-m3).
        /// - llm: relevance judge via the configured --llm-provider.
        #[arg(long, value_enum)]
        reranker: Option<RerankBackendKind>,

        /// Override LLM model name for `--reranker llm`. Defaults to the
        /// server's main `--llm-model`.
        #[arg(long, env = "RERANKER_MODEL")]
        reranker_model: Option<String>,

        /// Path to PageIndex workspace directory (enables book chat feature)
        #[arg(long, env = "PAGEINDEX_DIR", default_value = "data/pageindex")]
        pageindex_dir: Option<String>,
    },
}

fn main() -> Result<()> {
    let stack_size = 64 * 1024 * 1024; // 128 MB — SurrealDB recursive operations need deep stacks
    eprintln!("Starting tokio runtime with {stack_size} byte worker thread stacks");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(stack_size)
        .build()?
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hadith=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest {
            file,
            limit,
            translate,
            translate_model,
            db_path,
            embed_model,
        } => {
            if !std::path::Path::new(&file).exists() {
                anyhow::bail!(
                    "SemanticHadith JSON not found at {file}\n\
                     Run: make semantic-download && make semantic-extract"
                );
            }

            let db = db::connect(&db_path).await?;
            db::init_schema(&db, embed_model.dimension()).await?;
            db::init_fulltext_indexes(&db).await?;

            #[cfg(feature = "advanced")]
            let embedder = {
                let e = embed_model.build()?;
                embed::check_embedding_dimension(&db, e.as_ref()).await?;
                Some(e)
            };
            #[cfg(not(feature = "advanced"))]
            let embedder: Option<
                std::sync::Arc<dyn hadith::embedding::EmbeddingProvider>,
            > = {
                println!("Advanced features disabled — skipping embeddings");
                None
            };

            ingest::semantic::ingest(&db, &file, limit, embedder.as_deref()).await?;

            // Merge human English translations from sunnah.com (better quality)
            println!("🌐 Merging human English translations from sunnah.com...");
            ingest::sanadset::merge_human_translations(&db).await?;

            if translate {
                println!("🤖 Filling translation gaps via Ollama ({translate_model})...");
                ingest::sanadset::translate_all(&db, &translate_model).await?;
            }

            tracing::info!("Ingestion complete");
        }
        #[allow(unused_variables)]
        Commands::Analyze {
            db_path,
            families,
            mustalah,
            max_family_size,
            embed_model,
        } => {
            let db = db::connect(&db_path).await?;
            db::init_schema(&db, embed_model.dimension()).await?;

            #[allow(unused_mut)]
            let mut did_something = false;

            if families {
                #[cfg(feature = "advanced")]
                {
                    let embedder = embed_model.build()?;
                    let count = analysis::family::compute_families(&db, embedder.as_ref()).await?;
                    tracing::info!("Created {count} hadith families");
                    did_something = true;
                }
                #[cfg(not(feature = "advanced"))]
                {
                    tracing::warn!(
                        "Family analysis requires advanced features (embeddings). Build with `advanced` feature enabled."
                    );
                }
            }

            if mustalah {
                #[cfg(feature = "advanced")]
                {
                    println!("📖 Running mustalah al-hadith analysis...");
                    use surrealdb::types::{RecordId, SurrealValue};

                    #[derive(Debug, SurrealValue)]
                    struct MFamilyRow {
                        id: Option<RecordId>,
                        variant_count: Option<i64>,
                    }
                    let mut res = db
                    .query(
                        "SELECT id, variant_count FROM hadith_family ORDER BY variant_count DESC",
                    )
                    .await?;
                    let family_rows: Vec<MFamilyRow> = res.take(0)?;

                    // Resume support: check already-analyzed families
                    #[derive(Debug, SurrealValue)]
                    struct MFamilyRef {
                        family: RecordId,
                    }
                    let mut done_res = db.query("SELECT family FROM isnad_analysis").await?;
                    let done_rows: Vec<MFamilyRef> = done_res.take(0)?;
                    let already_done: std::collections::HashSet<String> = done_rows
                        .iter()
                        .map(|r| hadith::models::record_id_key_string(&r.family))
                        .collect();
                    let remaining = family_rows.len() - already_done.len().min(family_rows.len());
                    if !already_done.is_empty() {
                        println!(
                            "   Resuming: {remaining} families remaining ({} already done)",
                            already_done.len()
                        );
                    }

                    let pb = indicatif::ProgressBar::new(remaining as u64);
                    pb.set_style(
                        indicatif::ProgressStyle::default_bar()
                            .template("   {bar:40.green/black} {pos}/{len} families ({eta})")
                            .unwrap(),
                    );

                    let mut breadth_counts: std::collections::HashMap<String, usize> =
                        std::collections::HashMap::new();
                    let mut analyzed = 0usize;

                    for row in &family_rows {
                        let fid = row
                            .id
                            .as_ref()
                            .map(hadith::models::record_id_key_string)
                            .unwrap_or_default();
                        if fid.is_empty() || already_done.contains(&fid) {
                            continue;
                        }
                        let vcount = row.variant_count.unwrap_or(0) as usize;
                        if vcount > max_family_size {
                            pb.inc(1);
                            continue;
                        }

                        if let Some(result) =
                            analysis::mustalah::analyze_family_mustalah(&db, &fid).await?
                        {
                            let breadth_key =
                                format!("{:?}", result.breadth.classification).to_lowercase();
                            *breadth_counts.entry(breadth_key).or_default() += 1;
                            analysis::mustalah::store_mustalah_results(&db, &result).await?;
                            analyzed += 1;
                        }
                        pb.inc(1);
                    }
                    pb.finish_and_clear();

                    println!("   Mustalah analysis: {analyzed} families analyzed");
                    for (breadth, count) in &breadth_counts {
                        println!("     {breadth}: {count}");
                    }
                    tracing::info!("Mustalah analysis complete");
                    did_something = true;
                }
                #[cfg(not(feature = "advanced"))]
                {
                    tracing::warn!(
                        "Mustalah analysis requires advanced features. Build with `advanced` feature enabled."
                    );
                }
            }

            if !did_something {
                tracing::warn!("No analysis flags specified. Use --families or --mustalah.");
            }
        }
        Commands::IngestQuran {
            file,
            db_path,
            embed_model,
        } => {
            if !std::path::Path::new(&file).exists() {
                anyhow::bail!(
                    "Quran CSV not found at {file}. Run: python scripts/prepare_quran_data.py"
                );
            }

            let db = db::connect(&db_path).await?;
            let dim = embed_model.dimension();
            db::init_schema(&db, dim).await?;
            db::init_quran_schema(&db, dim).await?;
            db::init_quran_fulltext_indexes(&db).await?;
            #[cfg(feature = "advanced")]
            let embedder = {
                let e = embed_model.build()?;
                embed::check_embedding_dimension(&db, e.as_ref()).await?;
                Some(e)
            };
            #[cfg(not(feature = "advanced"))]
            let embedder: Option<
                std::sync::Arc<dyn hadith::embedding::EmbeddingProvider>,
            > = {
                println!("Advanced features disabled — skipping ayah embeddings");
                None
            };

            quran::ingest::ingest(&db, &file, embedder.as_deref()).await?;

            tracing::info!("Quran ingestion complete");
        }
        Commands::IngestQuranHadithRefs { db_path } => {
            let db = db::connect(&db_path).await?;
            db::init_schema(&db, FastembedModelKind::default().dimension()).await?;
            db::init_quran_schema(&db, FastembedModelKind::default().dimension()).await?;
            quran::hadith_refs::ingest_hadith_refs(&db).await?;
            tracing::info!("Quran-Hadith reference ingestion complete");
        }
        Commands::IngestMorphology {
            file,
            qul_dir,
            db_path,
        } => {
            if !std::path::Path::new(&file).exists() {
                anyhow::bail!(
                    "Morphology data not found at {file}. Download from: https://github.com/mustafa0x/quran-morphology"
                );
            }

            let db = db::connect(&db_path).await?;
            db::init_quran_word_schema(&db).await?;
            quran::morphology::ingest_morphology(&db, &file, &qul_dir).await?;
            quran::morphology::build_ayah_lemma_text(&db).await?;
            tracing::info!("Morphology ingestion complete");
        }
        Commands::IngestQuranSimilar { qul_dir, db_path } => {
            let db = db::connect(&db_path).await?;
            db::init_quran_schema(&db, FastembedModelKind::default().dimension()).await?;
            db::init_quran_word_schema(&db).await?;
            db::init_quran_similar_schema(&db).await?;
            println!("Ingesting shared phrases and similar ayahs from {qul_dir}...");
            quran::similar::ingest_similar(&db, &qul_dir).await?;
            tracing::info!("Quran similar/mutashabihat ingestion complete");
        }
        Commands::IngestTurath {
            pages_file,
            headings_file,
            book_id,
            name_ar,
            name_en,
            author_ar,
            tafsir_mapping,
            sharh_mapping,
            sharh_collection_id,
            narrator_mapping,
            category,
            book_type,
            force,
            db_path,
        } => {
            let db = db::connect(&db_path).await?;

            if force {
                tracing::info!("Force mode: clearing existing data for book {book_id}");
                let _ = db
                    .query(format!("DELETE book_page WHERE book_id = {book_id}"))
                    .await;
                let _ = db
                    .query(format!("DELETE book WHERE book_id = {book_id}"))
                    .await;
                let _ = db
                    .query(format!("DELETE tafsir_ayah_map WHERE book_id = {book_id}"))
                    .await;
                let _ = db
                    .query(format!("DELETE hadith_sharh_map WHERE book_id = {book_id}"))
                    .await;
                let _ = db
                    .query(format!(
                        "DELETE narrator_book_map WHERE book_id = {book_id}"
                    ))
                    .await;
            }

            ingest::book::ingest_book(
                &db,
                &pages_file,
                &headings_file,
                book_id,
                &name_ar,
                &name_en,
                &author_ar,
                category.as_deref(),
                book_type.as_deref(),
                None,
                None,
            )
            .await?;

            if let Some(tafsir_file) = tafsir_mapping {
                ingest::book::ingest_tafsir_mapping(&db, &tafsir_file, book_id).await?;
            }

            if let Some(sharh_file) = sharh_mapping {
                let collection_id = sharh_collection_id
                    .expect("--sharh-collection-id required when --sharh-mapping is provided");
                ingest::book::ingest_hadith_sharh_mapping(&db, &sharh_file, collection_id, book_id)
                    .await?;
            }

            if let Some(narrator_file) = narrator_mapping {
                ingest::book::ingest_narrator_book_mapping(&db, &narrator_file, book_id, &name_en)
                    .await?;
            }

            tracing::info!("Book ingestion complete for book {book_id}");
        }
        Commands::Serve {
            port,
            db_path,
            llm_provider,
            llm_model,
            llm_base_url,
            llm_api_key,
            embed_provider,
            embed_model,
            embed_base_url,
            embed_api_key,
            embed_dimensions,
            reranker,
            reranker_model,
            pageindex_dir,
        } => {
            let db = db::connect(&db_path).await?;

            // Embedding config is OPTIONAL. Omit --embed-model for lite mode
            // (text-only search). When set, build the matching `EmbedConfig`.
            let embed_cfg: Option<EmbedConfig> = embed_model.map(|model| EmbedConfig {
                provider: embed_provider,
                model,
                base_url: embed_base_url,
                api_key: embed_api_key,
                dimensions: embed_dimensions,
            });

            // The DB schema needs a fixed embedding dim. With no embedder
            // configured we still default to the lite-mode E5-small dim (384)
            // — same as the old behaviour, lets text-only search work over
            // databases that may or may not contain embeddings.
            let dim = match embed_cfg.as_ref() {
                None => FastembedModelKind::default().dimension(),
                Some(cfg) => match (cfg.provider, cfg.dimensions) {
                    (_, Some(d)) => d,
                    (EmbedProviderKind::Fastembed, None) => match cfg.model.as_str() {
                        "bge-m3" => 1024,
                        _ => 384,
                    },
                    (EmbedProviderKind::Openai, None) => match cfg.model.as_str() {
                        "text-embedding-3-large" => 3072,
                        _ => 1536,
                    },
                    (EmbedProviderKind::Ollama, None) => anyhow::bail!(
                        "--embed-dimensions / EMBED_DIMENSIONS is required when --embed-provider=ollama"
                    ),
                },
            };

            db::init_schema(&db, dim).await?;
            db::init_quran_schema(&db, dim).await?;
            db::init_quran_word_schema(&db).await?;
            db::init_quran_similar_schema(&db).await?;
            db::init_reciter_schema(&db).await?;
            db::init_book_schema(&db).await?;
            db::init_user_note_schema(&db).await?;
            db::init_link_preview_schema(&db).await?;
            db::init_notebook_schema(&db).await?;
            quran::audio::init_reciters(&db).await?;

            // LLM config is OPTIONAL. Omit --llm-model for lite mode
            // (no Ask, no classification, no LLM rerank).
            let llm_cfg: Option<LlmConfig> = llm_model.map(|model| LlmConfig {
                provider: llm_provider,
                model,
                base_url: llm_base_url,
                api_key: llm_api_key,
            });

            let serve_cfg = ServeConfig {
                port,
                llm: llm_cfg,
                embed: embed_cfg,
                rerank_backend: reranker,
                reranker_model,
                pageindex_dir,
            };

            web::serve(db, serve_cfg).await?;
        }
    }

    Ok(())
}
