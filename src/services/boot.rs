//! Shared boot helpers — DB connect + schema init.
//!
//! Used by both the `serve` and `mcp` CLI arms so the initialization sequence
//! is defined exactly once.

use anyhow::Result;
use surrealdb::Surreal;

use crate::db::{self, Db};

/// Connect to a SurrealKv database at `db_path` and initialize every schema
/// the running server needs (hadith, quran, books, gradings, notes, etc.).
///
/// `embed_dim` sets the vector dimension on the HNSW indexes — must match the
/// dimension of any embeddings already stored in the DB. Pass the lite-mode
/// default (384) when no embedder is configured.
pub async fn init_database(db_path: &str, embed_dim: usize) -> Result<Surreal<Db>> {
    let db = db::connect(db_path).await?;

    db::init_schema(&db, embed_dim).await?;
    db::init_quran_schema(&db, embed_dim).await?;
    db::init_quran_word_schema(&db).await?;
    db::init_quran_similar_schema(&db).await?;
    db::init_reciter_schema(&db).await?;
    db::init_book_schema(&db).await?;
    db::init_grading_schema(&db).await?;
    db::init_user_note_schema(&db).await?;
    db::init_link_preview_schema(&db).await?;
    db::init_notebook_schema(&db).await?;
    crate::quran::audio::init_reciters(&db).await?;

    Ok(db)
}
