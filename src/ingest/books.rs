//! Canonical hadith collections (Kutub al-Sittah) — single source of truth
//! for prefix / Arabic name / slug code / collection_id mappings used by
//! both ingest paths.

use crate::ingest::sanadset::normalize_arabic;

pub struct BookInfo {
    /// 2-letter prefix used in SemanticHadith JSON (e.g. "SB").
    pub prefix: &'static str,
    /// Canonical Arabic name (e.g. "صحيح البخاري").
    pub arabic_name: &'static str,
    /// URL/slug code used in hadith RecordIds (e.g. "bukhari").
    pub code: &'static str,
    /// Stable numeric id used in DB records.
    pub collection_id: i64,
}

pub const BOOKS: &[BookInfo] = &[
    BookInfo {
        prefix: "SB",
        arabic_name: "صحيح البخاري",
        code: "bukhari",
        collection_id: 1,
    },
    BookInfo {
        prefix: "SM",
        arabic_name: "صحيح مسلم",
        code: "muslim",
        collection_id: 2,
    },
    BookInfo {
        prefix: "SD",
        arabic_name: "سنن أبي داود",
        code: "abudawud",
        collection_id: 3,
    },
    BookInfo {
        prefix: "JT",
        arabic_name: "جامع الترمذي",
        code: "tirmidhi",
        collection_id: 4,
    },
    BookInfo {
        prefix: "SN",
        arabic_name: "سنن النسائى الصغرى",
        code: "nasai",
        collection_id: 5,
    },
    BookInfo {
        prefix: "IM",
        arabic_name: "سنن ابن ماجه",
        code: "ibnmajah",
        collection_id: 6,
    },
];

pub fn by_prefix(prefix: &str) -> Option<&'static BookInfo> {
    let p = prefix.to_ascii_uppercase();
    BOOKS.iter().find(|b| b.prefix == p)
}

pub fn by_arabic_name(name: &str) -> Option<&'static BookInfo> {
    let n = normalize_arabic(name);
    BOOKS.iter().find(|b| normalize_arabic(b.arabic_name) == n)
}

/// Hadith RecordId slug: e.g. `hadith_slug("bukhari", 1)` → `"bukhari:1"`.
pub fn hadith_slug(code: &str, num: i64) -> String {
    format!("{}:{}", code, num)
}

/// Tahwil variant suffix: e.g. `variant_slug("bukhari:1", 2)` → `"bukhari:1:v2"`.
pub fn variant_slug(base: &str, n: usize) -> String {
    format!("{}:v{}", base, n)
}
