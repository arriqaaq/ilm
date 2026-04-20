//! Book chat module — PageIndex-style agentic retrieval over Turath books.
//!
//! At startup, loads tree structures (built offline by PageIndex from markdown)
//! from disk. At query time:
//!   1. Two-phase navigation: pick chapter → pick section (~1K tokens each)
//!   2. Reads the text content from the tree nodes for those sections
//!   3. Sends the text + question to Ollama → streams the answer

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::rag::OllamaClient;

/// Truncate a string at a char boundary, not in the middle of a multi-byte character.
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

// ── Data structures ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BookTree {
    pub book_id: u64,
    pub name_en: String,
    pub name_ar: String,
    pub structure: serde_json::Value,
    pub line_count: usize,
    pub md_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionRange {
    pub start_line: u64,
    pub end_line: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionContent {
    pub line: u64,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookSource {
    pub line: u64,
    pub title: String,
}

// ── Navigation cache ────────────────────────────────────────────────────────

const CACHE_TTL_SECS: u64 = 600; // 10 minutes
const CACHE_MAX_ENTRIES: usize = 100;

type NavCacheEntries = HashMap<(u64, String), (Instant, Vec<SectionRange>)>;

pub struct NavCache {
    entries: Mutex<NavCacheEntries>,
}

impl Default for NavCache {
    fn default() -> Self {
        Self::new()
    }
}

impl NavCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, book_id: u64, question: &str) -> Option<Vec<SectionRange>> {
        let entries = self.entries.lock().ok()?;
        let key = (book_id, question.to_string());
        if let Some((instant, ranges)) = entries.get(&key)
            && instant.elapsed().as_secs() < CACHE_TTL_SECS
        {
            return Some(ranges.clone());
        }
        None
    }

    pub fn put(&self, book_id: u64, question: &str, ranges: Vec<SectionRange>) {
        if let Ok(mut entries) = self.entries.lock() {
            // Evict expired entries if at capacity
            if entries.len() >= CACHE_MAX_ENTRIES {
                entries.retain(|_, (instant, _)| instant.elapsed().as_secs() < CACHE_TTL_SECS);
            }
            // If still at capacity, clear oldest half
            if entries.len() >= CACHE_MAX_ENTRIES {
                let mut by_age: Vec<_> = entries.keys().cloned().collect();
                by_age
                    .sort_by_key(|k| entries.get(k).map(|(i, _)| i.elapsed()).unwrap_or_default());
                // Remove oldest half
                for key in by_age.iter().rev().take(CACHE_MAX_ENTRIES / 2) {
                    entries.remove(key);
                }
            }
            entries.insert((book_id, question.to_string()), (Instant::now(), ranges));
        }
    }
}

// ── Book map JSON (written by scripts/index_books.py) ───────────────────────

#[derive(Debug, Deserialize)]
struct BookMapEntry {
    name_en: String,
    #[serde(default)]
    name_ar: String,
    #[serde(default)]
    line_count: usize,
    #[serde(default)]
    md_path: String,
}

// ── Loading ─────────────────────────────────────────────────────────────────

/// Load all book trees from the PageIndex workspace directory.
pub fn load_book_trees(workspace_dir: &Path) -> Result<HashMap<u64, BookTree>> {
    let book_map_path = workspace_dir.join("book_map.json");
    if !book_map_path.exists() {
        anyhow::bail!(
            "book_map.json not found in {}. Run: python3 scripts/index_books.py",
            workspace_dir.display()
        );
    }

    let raw = std::fs::read_to_string(&book_map_path)
        .with_context(|| format!("reading {}", book_map_path.display()))?;
    let book_map: HashMap<String, BookMapEntry> =
        serde_json::from_str(&raw).context("parsing book_map.json")?;

    let mut trees = HashMap::new();

    for (book_id_str, entry) in &book_map {
        let book_id: u64 = book_id_str
            .parse()
            .with_context(|| format!("invalid book_id in book_map.json: {book_id_str}"))?;

        let tree_path = workspace_dir.join(format!("{book_id_str}.json"));
        if !tree_path.exists() {
            tracing::warn!(
                "Tree file {}.json not found for book {}, skipping",
                book_id_str,
                entry.name_en
            );
            continue;
        }

        let tree_raw = std::fs::read_to_string(&tree_path)
            .with_context(|| format!("reading {}", tree_path.display()))?;
        let tree_doc: serde_json::Value = serde_json::from_str(&tree_raw)
            .with_context(|| format!("parsing {}", tree_path.display()))?;

        let structure = tree_doc
            .get("structure")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![]));

        let line_count = tree_doc
            .get("line_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(entry.line_count as u64) as usize;

        let md_path = PathBuf::from(&entry.md_path);

        tracing::info!(
            "Loaded book {} ({}) — {} lines",
            book_id,
            entry.name_en,
            line_count,
        );

        trees.insert(
            book_id,
            BookTree {
                book_id,
                name_en: entry.name_en.clone(),
                name_ar: entry.name_ar.clone(),
                structure,
                line_count,
                md_path,
            },
        );
    }

    Ok(trees)
}

// ── TOC formatting ──────────────────────────────────────────────────────────

/// Rough token estimate (~3 chars/token for mixed Arabic/English + JSON punctuation).
fn estimate_tokens(s: &str) -> usize {
    s.len() / 3
}

/// Recursively emit a flat indented TOC of the whole tree (title + line_num per node).
/// Matches PageIndex's `get_document_structure` semantics — the LLM sees the full
/// hierarchy in one compact serialization.
fn format_full_toc(structure: &serde_json::Value) -> String {
    let mut out = String::new();
    walk_toc(structure, 0, &mut out);
    out
}

fn walk_toc(node: &serde_json::Value, depth: usize, out: &mut String) {
    if let Some(arr) = node.as_array() {
        for child in arr {
            walk_toc(child, depth, out);
        }
        return;
    }
    let title = node
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("(untitled)");
    if let Some(ln) = node.get("line_num").and_then(|v| v.as_u64()) {
        let indent = "  ".repeat(depth);
        out.push_str(&format!("{indent}{title} [line {ln}]\n"));
    }
    if let Some(children) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in children {
            walk_toc(child, depth + 1, out);
        }
    }
}

/// Split the full TOC into slabs whose individual token cost stays under `budget`.
/// Boundaries fall on top-level chapter (level-1) headings so each slab is a
/// coherent run of kutub, not a mid-chapter cut.
fn split_toc_into_slabs(structure: &serde_json::Value, budget: usize) -> Vec<String> {
    let chapters = structure
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|root| root.get("nodes").and_then(|v| v.as_array()));

    let Some(chapters) = chapters else {
        return vec![format_full_toc(structure)];
    };

    let mut slabs: Vec<String> = Vec::new();
    let mut current = String::new();

    for ch in chapters {
        let mut ch_toc = String::new();
        walk_toc(ch, 0, &mut ch_toc);

        if !current.is_empty() && estimate_tokens(&current) + estimate_tokens(&ch_toc) > budget {
            slabs.push(std::mem::take(&mut current));
        }
        current.push_str(&ch_toc);
    }
    if !current.is_empty() {
        slabs.push(current);
    }
    if slabs.is_empty() {
        slabs.push(format_full_toc(structure));
    }
    slabs
}

/// Collect all line_num values in the entire tree (for validation).
fn collect_all_line_nums(node: &serde_json::Value, out: &mut std::collections::HashSet<u64>) {
    if let Some(arr) = node.as_array() {
        for child in arr {
            collect_all_line_nums(child, out);
        }
        return;
    }
    if let Some(ln) = node.get("line_num").and_then(|v| v.as_u64()) {
        out.insert(ln);
    }
    if let Some(children) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in children {
            collect_all_line_nums(child, out);
        }
    }
}

/// Parse `{"ranges": [...]}` — accepts strings like `"15415-15500"`, bare numbers,
/// or `{start_line, end_line}` objects. Invalid entries are skipped.
fn parse_ranges(v: &serde_json::Value) -> Vec<SectionRange> {
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };

    arr.iter()
        .take(5)
        .filter_map(|item| {
            if let Some(s) = item.as_str() {
                return parse_range_str(s);
            }
            let start = item
                .get("start_line")
                .and_then(|v| v.as_u64())
                .or_else(|| item.get("start").and_then(|v| v.as_u64()))?;
            let end = item
                .get("end_line")
                .and_then(|v| v.as_u64())
                .or_else(|| item.get("end").and_then(|v| v.as_u64()))
                .unwrap_or(start + 200);
            Some(SectionRange {
                start_line: start,
                end_line: if end >= start { end } else { start + 200 },
            })
        })
        .collect()
}

fn parse_range_str(s: &str) -> Option<SectionRange> {
    let trimmed = s.trim();
    if let Some((a, b)) = trimmed.split_once('-') {
        let start: u64 = a.trim().parse().ok()?;
        let end: u64 = b.trim().parse().ok()?;
        if end >= start {
            return Some(SectionRange {
                start_line: start,
                end_line: end,
            });
        }
    }
    if let Ok(n) = trimmed.parse::<u64>() {
        return Some(SectionRange {
            start_line: n,
            end_line: n + 200,
        });
    }
    None
}

// ── Navigation ──────────────────────────────────────────────────────────────

/// Token budget per LLM call. If the full TOC fits, we do one call exactly like
/// PageIndex's reference flow. If it doesn't, we split into parallel batches.
const BATCH_TOKEN_BUDGET: usize = 80_000;

/// Navigate the book tree to pick line ranges relevant to `question`.
/// Single LLM call when the TOC fits, else N parallel calls merged.
pub async fn navigate(
    ollama: &OllamaClient,
    book: &BookTree,
    question: &str,
) -> Result<Vec<SectionRange>> {
    let full_toc = format_full_toc(&book.structure);
    let total_tokens = estimate_tokens(&full_toc);

    let mut valid_lines: std::collections::HashSet<u64> = std::collections::HashSet::new();
    collect_all_line_nums(&book.structure, &mut valid_lines);

    if total_tokens <= BATCH_TOKEN_BUDGET {
        tracing::info!(
            "navigate: 1 batch ({}K tokens) for {}",
            total_tokens / 1000,
            book.name_en
        );
        return navigate_once(ollama, book, question, &full_toc, &valid_lines).await;
    }

    let slabs = split_toc_into_slabs(&book.structure, BATCH_TOKEN_BUDGET);
    tracing::info!(
        "navigate: {} batches parallel ({}K tokens total) for {}",
        slabs.len(),
        total_tokens / 1000,
        book.name_en
    );

    let results = futures::future::join_all(
        slabs
            .iter()
            .map(|slab| navigate_once(ollama, book, question, slab, &valid_lines)),
    )
    .await;

    let mut merged: Vec<SectionRange> = Vec::new();
    for r in results.into_iter().flatten() {
        for range in r {
            if !merged.iter().any(|m| m.start_line == range.start_line) {
                merged.push(range);
            }
        }
    }
    merged.truncate(5);
    Ok(merged)
}

async fn navigate_once(
    ollama: &OllamaClient,
    book: &BookTree,
    question: &str,
    toc: &str,
    valid_lines: &std::collections::HashSet<u64>,
) -> Result<Vec<SectionRange>> {
    let system = format!(
        "You are navigating the table of contents of \"{name}\".\n\
         Identify 1-3 line ranges most likely to contain the answer to the user's question.\n\
         Return JSON only: {{\"ranges\": [\"start-end\", ...]}}\n\
         Rules:\n\
         - Each range is a string like \"15415-15500\" using line numbers shown in brackets\n\
         - Use line numbers that ACTUALLY appear in the TOC below\n\
         - Match on topic/meaning, not exact wording — questions may use different terms\n\n\
         Table of Contents:\n{toc}",
        name = book.name_en,
    );

    let result = ollama
        .chat_json(&system, question, None)
        .await
        .context("navigate_once (LLM range selection) failed")?;

    let ranges = parse_ranges(result.get("ranges").unwrap_or(&serde_json::Value::Null));

    // Validate: keep only ranges whose start_line exists in the tree
    let validated: Vec<SectionRange> = ranges
        .into_iter()
        .filter(|r| {
            if valid_lines.contains(&r.start_line) {
                true
            } else {
                tracing::warn!(
                    "LLM returned non-existent start_line {}, dropping",
                    r.start_line
                );
                false
            }
        })
        .collect();

    Ok(validated)
}

// ── Section text fetching ───────────────────────────────────────────────────

/// Fetch section text from the tree's embedded text content. Mirrors PageIndex's
/// `_get_md_page_content`: flat walk of the tree, keep nodes whose line_num
/// falls in [start, end], dedupe, sort by line.
pub fn fetch_sections(book: &BookTree, ranges: &[SectionRange]) -> Result<Vec<SectionContent>> {
    let mut results = Vec::new();
    for range in ranges {
        collect_sections_in_range(
            &book.structure,
            range.start_line,
            range.end_line,
            &mut results,
        );
    }
    let mut seen = std::collections::HashSet::new();
    results.retain(|s| seen.insert(s.line));
    results.sort_by_key(|s| s.line);
    Ok(results)
}

fn collect_sections_in_range(
    node: &serde_json::Value,
    start: u64,
    end: u64,
    results: &mut Vec<SectionContent>,
) {
    if let Some(arr) = node.as_array() {
        for child in arr {
            collect_sections_in_range(child, start, end, results);
        }
        return;
    }

    let line_num = node.get("line_num").and_then(|v| v.as_u64()).unwrap_or(0);

    if line_num >= start && line_num <= end {
        let title = node
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let text = node
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !text.is_empty() {
            results.push(SectionContent {
                line: line_num,
                title,
                text,
            });
        }
    }

    if let Some(children) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in children {
            collect_sections_in_range(child, start, end, results);
        }
    }
}

// ── Answer generation context builder ───────────────────────────────────────

/// Build the system prompt with section excerpts for the answer generation step.
pub fn build_answer_prompt(book_name: &str, sections: &[SectionContent]) -> String {
    let mut context = String::new();
    for s in sections {
        context.push_str(&format!(
            "--- [{title}] (line {line}) ---\n{text}\n\n",
            title = s.title,
            line = s.line,
            text = s.text
        ));
    }

    // Cap context at ~25K bytes (find valid char boundary)
    if context.len() > 25_000 {
        let safe = truncate_str(&context, 25_000).len();
        context.truncate(safe);
        context.push_str("\n... (content truncated)\n");
    }

    format!(
        "You are a knowledgeable Islamic scholar answering questions about \"{book_name}\".\n\
         Use ONLY the section excerpts provided below as context.\n\
         Always cite your sources by mentioning the section title when referencing specific content.\n\
         If the excerpts don't contain relevant information, say so honestly.\n\
         Respond in the same language as the user's question.\n\
         Be concise and accurate.\n\n\
         ## Section Excerpts:\n\n{context}"
    )
}

// ── Extractive tafsir synthesis ────────────────────────────────────────────
//
// /api/tafsir/ask runs an *extractive* prompt per book: the model selects
// verbatim Arabic passages from the provided pages and writes a short
// explanation per passage. It does NOT paraphrase, and it may only cite
// books from an explicit allow-list. Every output entry is verified
// server-side before reaching the client — quotes that aren't a substring
// of the actual page (after normalization) are dropped and counted, as are
// entries pointing at unknown book_ids.
//
// See `build_tafsir_extract_prompt`, `validate_extract_result`.

/// Normalize Arabic for substring comparison. Strips tatweel (U+0640) and
/// tashkeel (U+064B..U+0652) — which models add or drop arbitrarily — and
/// collapses whitespace so newlines and multiple spaces between words don't
/// break the match.
fn normalize_arabic(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c == '\u{0640}' || ('\u{064B}'..='\u{0652}').contains(&c) {
            continue;
        }
        if c.is_whitespace() {
            if !prev_ws && !out.is_empty() {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            out.push(c);
            prev_ws = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Verify that `quote` appears verbatim (modulo normalization) inside
/// `haystack`. This is the anti-hallucination guard — if the model invents
/// a quote, or paraphrases, or translates, the normalized substring will
/// not match and the entry is dropped.
pub fn verify_quote(quote: &str, haystack: &str) -> bool {
    let q = normalize_arabic(quote);
    if q.is_empty() {
        return false;
    }
    normalize_arabic(haystack).contains(&q)
}

// ── Structured output shapes ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RawExtractEntry {
    #[serde(default)]
    book_id: Option<u64>,
    #[serde(default)]
    page_index: Option<u64>,
    #[serde(default)]
    arabic_quote: Option<String>,
    #[serde(default)]
    english_note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawExtract {
    #[serde(default)]
    overview: Option<String>,
    #[serde(default)]
    entries: Vec<RawExtractEntry>,
}

/// One validated extract entry. All fields are guaranteed non-empty and
/// `arabic_quote` has been verified against the actual page text.
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedEntry {
    pub book_id: u64,
    pub page_index: u64,
    pub arabic_quote: String,
    pub english_note: String,
}

/// The server's trust boundary: only fields in here are forwarded to the
/// client. `dropped` is the count of raw-LLM entries that failed validation
/// (unknown book_id, unknown page, or quote not verbatim).
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedExtract {
    pub overview: Option<String>,
    pub entries: Vec<ValidatedEntry>,
    pub dropped: usize,
}

/// Parse the raw JSON returned by the extractive prompt, then validate each
/// entry against the allow-list of book_ids and the actual page texts we
/// fed in. Malformed JSON → empty extract (never panics). Per-entry
/// failures are logged and counted in `dropped`, never surfaced as errors —
/// we still want to return the good entries alongside a drop count.
pub fn validate_extract_result(
    raw: serde_json::Value,
    allowed_book_ids: &std::collections::HashSet<u64>,
    page_texts: &std::collections::HashMap<(u64, u64), String>,
) -> ValidatedExtract {
    let parsed: RawExtract = serde_json::from_value(raw).unwrap_or(RawExtract {
        overview: None,
        entries: Vec::new(),
    });

    let mut entries: Vec<ValidatedEntry> = Vec::new();
    let mut dropped = 0usize;

    for e in parsed.entries {
        let Some(book_id) = e.book_id else {
            dropped += 1;
            continue;
        };
        let Some(page_index) = e.page_index else {
            dropped += 1;
            continue;
        };
        let Some(arabic_quote) = e.arabic_quote else {
            dropped += 1;
            continue;
        };
        let english_note = e.english_note.unwrap_or_default();

        if arabic_quote.trim().is_empty() {
            dropped += 1;
            continue;
        }
        if !allowed_book_ids.contains(&book_id) {
            tracing::warn!(
                "tafsir extract: dropped entry with disallowed book_id {book_id} \
                 (allowed: {:?})",
                allowed_book_ids
            );
            dropped += 1;
            continue;
        }
        let Some(page_text) = page_texts.get(&(book_id, page_index)) else {
            tracing::warn!(
                "tafsir extract: dropped entry with unknown (book {book_id}, page {page_index})"
            );
            dropped += 1;
            continue;
        };
        if !verify_quote(&arabic_quote, page_text) {
            tracing::warn!(
                "tafsir extract: dropped entry — quote not verbatim in (book {book_id}, \
                 page {page_index}). Quote starts: {:?}",
                arabic_quote.chars().take(40).collect::<String>()
            );
            dropped += 1;
            continue;
        }

        entries.push(ValidatedEntry {
            book_id,
            page_index,
            arabic_quote,
            english_note,
        });
    }

    ValidatedExtract {
        overview: parsed.overview,
        entries,
        dropped,
    }
}

/// Build the extractive prompt for /api/tafsir/ask verse-aware path. The
/// model is told the *exact* book_ids and scholar names it may cite, and
/// the JSON schema it must return. Page headers include the book_id and
/// page_index so the model can copy them into entries without invention.
///
/// `books`: `(book_id, display_name, pages)` triples. `pages` comes from
/// the verse-aware page fetch, *capped* at N pages per book by the caller
/// to keep the LLM context focused.
pub fn build_tafsir_extract_prompt(
    verse: (u64, u64),
    books: &[(u64, String, Vec<SectionContent>)],
) -> String {
    let allowed_names: Vec<String> = books.iter().map(|(_, n, _)| n.clone()).collect();
    let allowed_ids: Vec<String> = books.iter().map(|(id, _, _)| id.to_string()).collect();

    // Build per-page context. Each page is prefixed with machine-readable
    // metadata (book_id, page_index) so the model can echo them verbatim
    // into entries, plus a human label for its own reasoning.
    let mut context = String::new();
    for (book_id, name, pages) in books {
        for p in pages {
            context.push_str(&format!(
                "\n--- {name} · {title} · book_id={book_id} · page_index={page_index}\n{text}\n",
                name = name,
                title = p.title,
                book_id = book_id,
                page_index = p.line,
                text = p.text
            ));
        }
    }
    if context.len() > 25_000 {
        let safe = truncate_str(&context, 25_000).len();
        context.truncate(safe);
        context.push_str("\n… (content truncated)\n");
    }

    format!(
        "You are an extractive assistant for classical Qur'anic tafsir. The user is asking \
         about Qur'an verse {surah}:{ayah}. Answer by selecting VERBATIM Arabic passages \
         from the tafsir pages below and explaining in the user's language why each passage \
         answers the question.\n\n\
         STRICT RULES (each one violated = a failure):\n\
         1. The ONLY scholars and books available in this corpus are listed below: \
            {names} (book_ids: [{ids}]). Do NOT mention or cite ANY other scholar or \
            tafsir book — any attribution to a name not in that list is a fabrication \
            and will be rejected.\n\
         2. Every `arabic_quote` MUST be a verbatim substring of one of the pages below. \
            Copy the exact characters. Do NOT translate, paraphrase, summarize, or \"clean up\" \
            the Arabic. If no page has relevant Arabic, omit that book — do NOT invent a quote.\n\
         3. Use ONLY the exact `book_id` and `page_index` values shown in each page header. \
            Never make up page numbers.\n\
         4. Keep each `arabic_quote` focused — one to three sentences. 1–3 entries per book \
            is plenty. Prefer quality over quantity.\n\
         5. Write `english_note` in the SAME language the user used for the question. Keep \
            it 1–2 sentences, grounded in the passage you just quoted.\n\n\
         OUTPUT FORMAT — JSON ONLY, matching this shape exactly:\n\
         {{\n  \
           \"overview\": \"optional 1–2 sentence framing in the user's language, or null\",\n  \
           \"entries\": [\n    \
             {{\n      \
               \"book_id\": <number from allow-list>,\n      \
               \"page_index\": <number from a page header>,\n      \
               \"arabic_quote\": \"<verbatim substring of that page>\",\n      \
               \"english_note\": \"<short explanation>\"\n    \
             }}\n  \
           ]\n\
         }}\n\n\
         If none of the pages answer the question, return: {{\"overview\": null, \"entries\": []}}\n\n\
         === TAFSIR PAGES ===\n{context}\n=== END PAGES ===",
        surah = verse.0,
        ayah = verse.1,
        names = allowed_names.join(", "),
        ids = allowed_ids.join(", "),
    )
}

// ── Build sources from section ranges and tree structure ────────────────────

/// Convert section ranges into source citations.
pub fn build_sources(book: &BookTree, ranges: &[SectionRange]) -> Vec<BookSource> {
    let mut sources = Vec::new();

    for range in ranges {
        if let Some(title) = find_title_at_line(&book.structure, range.start_line) {
            sources.push(BookSource {
                line: range.start_line,
                title,
            });
        }
    }

    sources
}

fn find_title_at_line(node: &serde_json::Value, target_line: u64) -> Option<String> {
    if let Some(arr) = node.as_array() {
        for child in arr {
            if let Some(t) = find_title_at_line(child, target_line) {
                return Some(t);
            }
        }
        return None;
    }

    let line_num = node.get("line_num").and_then(|v| v.as_u64()).unwrap_or(0);
    if line_num == target_line {
        return node
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }

    if let Some(children) = node.get("nodes").and_then(|v| v.as_array()) {
        let mut best: Option<String> = None;
        for child in children {
            if let Some(t) = find_title_at_line(child, target_line) {
                return Some(t);
            }
            let child_line = child.get("line_num").and_then(|v| v.as_u64()).unwrap_or(0);
            if child_line <= target_line {
                best = child
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        if best.is_some() {
            return best;
        }
    }

    None
}
