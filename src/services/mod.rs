//! Domain service layer.
//!
//! Single source of truth for every domain operation in the corpus. Both the
//! HTTP layer (`crate::web`) and the MCP layer (`crate::mcp`) call into here;
//! neither contains business logic.
//!
//! Service rules of thumb:
//! - Take plain Rust args (no `Path`/`Query`/`Json`/`Parameters` extractors).
//! - Return canonical response shapes (`Api*` structs, tuples, or streams).
//! - Never reference `axum::*` or `rmcp::*`.

pub mod ask;
pub mod book;
pub mod boot;
pub mod classify;
pub mod family;
pub mod hadith;
pub mod meta;
pub mod narrator;
pub mod notes;
pub mod quran;
pub mod scholars;
pub mod search;
pub mod tafsir;
