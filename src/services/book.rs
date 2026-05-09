//! Book-domain services.
//!
//! Reads over ingested turath books (Tafsir, Sharh, narrator biographies,
//! hadith grading source books) — list, detail with TOC headings, page
//! windows for the reader.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use surrealdb::types::SurrealValue;
use utoipa::ToSchema;

use crate::web::AppState;

// ── Public response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct BookSummary {
    pub book_id: u64,
    pub name_ar: String,
    pub name_en: String,
    pub author_ar: String,
    pub total_pages: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BookDetail {
    pub book_id: u64,
    pub name_ar: String,
    pub name_en: String,
    pub author_ar: String,
    pub total_pages: u64,
    pub headings: Vec<BookHeading>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BookHeading {
    pub title: String,
    pub level: u32,
    pub page_index: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BookPage {
    pub page_index: u64,
    pub text: String,
    pub vol: String,
    pub page_num: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BookPagesResponse {
    pub pages: Vec<BookPage>,
    pub total: u64,
    pub start: u64,
    pub size: u64,
}

// ── DB read shapes ─────────────────────────────────────────────────────────

#[derive(Debug, SurrealValue)]
struct BookRow {
    book_id: i64,
    name_ar: String,
    name_en: String,
    author_ar: String,
    total_pages: i64,
    headings: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct PageRow {
    page_index: i64,
    text: String,
    vol: String,
    page_num: i64,
}

#[derive(Debug, SurrealValue)]
struct CountRow {
    c: i64,
}

// ── Services ───────────────────────────────────────────────────────────────

/// All ingested books, optionally filtered by `category` (coarse domain) and
/// `book_type` (genre/role). Both filters narrow when supplied together.
pub async fn list(
    state: &AppState,
    category: Option<&str>,
    book_type: Option<&str>,
) -> Result<Vec<BookSummary>> {
    let mut sql =
        String::from("SELECT book_id, name_ar, name_en, author_ar, total_pages, headings FROM book");
    let mut clauses: Vec<&str> = Vec::new();
    if category.is_some() {
        clauses.push("category = $cat");
    }
    if book_type.is_some() {
        clauses.push("book_type = $btype");
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }

    let mut q = state.db.query(&sql);
    if let Some(cat) = category {
        q = q.bind(("cat", cat.to_string()));
    }
    if let Some(bt) = book_type {
        q = q.bind(("btype", bt.to_string()));
    }
    let rows: Vec<BookRow> = q
        .await
        .context("book list query failed")?
        .take(0)
        .unwrap_or_default();
    Ok(rows
        .into_iter()
        .map(|b| BookSummary {
            book_id: b.book_id as u64,
            name_ar: b.name_ar,
            name_en: b.name_en,
            author_ar: b.author_ar,
            total_pages: b.total_pages as u64,
        })
        .collect())
}

/// One book's metadata + table-of-contents headings.
pub async fn get(state: &AppState, book_id: u64) -> Result<BookDetail> {
    let book: BookRow = state
        .db
        .query("SELECT * FROM book WHERE book_id = $bid LIMIT 1")
        .bind(("bid", book_id as i64))
        .await
        .context("book lookup query failed")?
        .take::<Option<BookRow>>(0)
        .unwrap_or(None)
        .ok_or_else(|| anyhow!("book not found: {book_id}"))?;

    let headings: Vec<BookHeading> = book
        .headings
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    Ok(BookDetail {
        book_id: book.book_id as u64,
        name_ar: book.name_ar,
        name_en: book.name_en,
        author_ar: book.author_ar,
        total_pages: book.total_pages as u64,
        headings,
    })
}

/// Page window for a book — `start..start+size` ordered by `page_index`.
/// Includes the total page count for pagination.
pub async fn get_pages(
    state: &AppState,
    book_id: u64,
    start: u64,
    size: u64,
) -> Result<BookPagesResponse> {
    let size = size.clamp(1, 100);
    let mut res = state
        .db
        .query(
            "SELECT page_index, text, vol, page_num FROM book_page \
             WHERE book_id = $bid AND page_index >= $start AND page_index < $end \
             ORDER BY page_index ASC",
        )
        .bind(("bid", book_id as i64))
        .bind(("start", start as i64))
        .bind(("end", (start + size) as i64))
        .await
        .context("book pages query failed")?;
    let pages: Vec<PageRow> = res.take(0).unwrap_or_default();

    let total: u64 = state
        .db
        .query("SELECT count() AS c FROM book_page WHERE book_id = $bid GROUP ALL")
        .bind(("bid", book_id as i64))
        .await
        .context("book page count query failed")?
        .take::<Option<CountRow>>(0)
        .unwrap_or(None)
        .map(|c| c.c as u64)
        .unwrap_or(0);

    Ok(BookPagesResponse {
        pages: pages
            .into_iter()
            .map(|p| BookPage {
                page_index: p.page_index as u64,
                text: p.text,
                vol: p.vol,
                page_num: p.page_num as u64,
            })
            .collect(),
        total,
        start,
        size,
    })
}

/// Distinguishes "book not found" so HTTP/MCP can surface 404 / `invalid_request`.
pub fn is_not_found(e: &anyhow::Error) -> bool {
    e.to_string().starts_with("book not found:")
}
