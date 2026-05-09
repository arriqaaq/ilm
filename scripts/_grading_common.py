"""Shared utilities for per-book grading extractors.

Each book (1147, 1148, 1216, 25881, 5914, 10757, 1663) has its own
extract_book_<id>_<slug>.py script. Anything shared between them lives here:
HTML stripping, Arabic-Indic digit conversion, page-offset bookkeeping, and
the corpus-presence check that confirms a (collection_id, hadith_number) pair
is present in our hadith dataset.

Why a thin shared module instead of a generalized parser: the user wants each
book's parser tuned to that book's actual edition format. Common helpers stay
mechanical (no parsing logic), so changes to one book's parser cannot cause
regressions in another.
"""

import json
import os
import re
import unicodedata

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_DIR = os.path.join(ROOT, "data")
SEMANTIC_HADITH_PATH = os.path.join(DATA_DIR, "semantic_hadith.json")

# Collection IDs match src/ingest/semantic.rs.
COLLECTION_TO_BOOK_CODE = {
    1: "SB",   # Sahih al-Bukhari
    2: "SM",   # Sahih Muslim
    3: "SD",   # Sunan Abi Dawud
    4: "JT",   # Jami at-Tirmidhi
    5: "SN",   # Sunan an-Nasa'i
    6: "IM",   # Sunan Ibn Majah
}

ARABIC_DIGITS = "٠١٢٣٤٥٦٧٨٩"
WESTERN_DIGITS = "0123456789"
DIGIT_TRANS = str.maketrans(ARABIC_DIGITS, WESTERN_DIGITS)


def strip_html(s: str) -> str:
    """Remove <span ...>, </span>, and any other markup that the Turath API
    embeds in page text. Leaves all Arabic content untouched."""
    return re.sub(r"<[^>]+>", "", s)


def strip_diacritics(s: str) -> str:
    """Strip Arabic diacritics (fathah, kasrah, dammah, sukun, shadda, etc.).
    Required because some Turath pages have stray diacritics inside verdict
    words (e.g. "صحَيح" instead of "صحيح") which break exact-match regexes."""
    return "".join(c for c in s if unicodedata.category(c) != "Mn")


def normalize_digits(s: str) -> str:
    """Convert Arabic-Indic digits (٠–٩) to ASCII (0–9)."""
    return s.translate(DIGIT_TRANS)


def load_pages(slug: str):
    """Load <slug>_pages.json into a list of records. Returns None if missing."""
    path = os.path.join(DATA_DIR, f"{slug}_pages.json")
    if not os.path.exists(path):
        return None
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def concatenate_pages(pages, *, strip_marks: bool = True):
    """Concatenate all stripped pages into one string + a (cum_offset, page_index)
    map so a parser can derive the source page_index for any character offset.

    `strip_marks=True` (default) also removes Arabic diacritics — required for
    reliable verdict-word matching. Set to False if you need verbatim text.

    Returns: (full_text, offsets) where offsets is a sorted list of
    (cum_offset, page_index_zero_based) tuples.
    """
    parts = []
    offsets = []
    cum = 0
    for p in pages:
        page_index = (p.get("page_id") or 1) - 1
        text = strip_html(p.get("text", ""))
        if strip_marks:
            text = strip_diacritics(text)
        offsets.append((cum, page_index))
        parts.append(text)
        cum += len(text) + 1  # +1 for "\n" separator
    return "\n".join(parts), offsets


def page_for_offset(offsets, off: int) -> int:
    """Map a character offset back to the page_index it came from."""
    last = 0
    for cum, pi in offsets:
        if cum > off:
            return last
        last = pi
    return last


# ── Corpus presence check ──

_corpus_index = None


def _build_corpus_index():
    """Build a set of (collection_id, hadith_number) pairs present in the
    project's hadith corpus. The semantic_hadith.json file is the source of
    truth for which hadiths exist (it's what src/ingest/semantic.rs ingests).
    """
    if not os.path.exists(SEMANTIC_HADITH_PATH):
        return set()
    with open(SEMANTIC_HADITH_PATH, "r", encoding="utf-8") as f:
        d = json.load(f)
    code_to_collection = {v: k for k, v in COLLECTION_TO_BOOK_CODE.items()}
    idx = set()
    for _, h in (d.get("hadiths") or {}).items():
        book_code = h.get("book")
        ref_no = h.get("refNo")
        if not book_code or ref_no is None:
            continue
        cid = code_to_collection.get(book_code)
        if cid is None:
            continue
        try:
            idx.add((cid, int(ref_no)))
        except (TypeError, ValueError):
            continue
    return idx


def in_corpus(collection_id: int, hadith_number: int) -> bool:
    """True iff (collection_id, hadith_number) is present in semantic_hadith.json."""
    global _corpus_index
    if _corpus_index is None:
        _corpus_index = _build_corpus_index()
    return (collection_id, hadith_number) in _corpus_index


def corpus_size_for_collection(collection_id: int) -> int:
    """Number of hadiths in the corpus for a given collection — useful for
    coverage ratio checks ("we extracted X of Y hadiths from this collection")."""
    global _corpus_index
    if _corpus_index is None:
        _corpus_index = _build_corpus_index()
    return sum(1 for (cid, _) in _corpus_index if cid == collection_id)


# ── Output helpers ──


_OPEN_PARENS = "(«"
_CLOSE_PARENS = ")»"


def find_balanced_paren_block(text: str, start: int = 0):
    """Walk forward from `start` to the first `(` or `«`, then return the
    (start, end, inner_text) of the balanced paren block.

    Handles two annoyances common in Turath OCR copies:
      - Some pages use `«` and `»` as alternate paren chars.
      - Some pages have `(` opened with a `»` close instead of `)`.

    Both `(` and `«` count as openers; both `)` and `»` count as closers.
    If the balance never reaches 0 within the next 4000 chars, gives up on
    this opener and skips past it (returns None for THIS opener; the caller
    should advance `start` past it and try again).
    """
    n = len(text)
    i = start
    while i < n and text[i] not in _OPEN_PARENS:
        i += 1
    if i >= n:
        return None
    block_start = i
    i += 1
    depth = 1
    max_scan = min(n, i + 4000)  # cap walk so unbalanced openers don't kill us
    while i < max_scan and depth > 0:
        c = text[i]
        if c in _OPEN_PARENS:
            depth += 1
        elif c in _CLOSE_PARENS:
            depth -= 1
        i += 1
    if depth != 0:
        return None  # unbalanced (caller should advance past block_start+1)
    return (block_start, i, text[block_start + 1:i - 1])


def iter_paren_blocks(text: str, start: int = 0):
    """Yield successive balanced-paren blocks. When an opener is unbalanced,
    skip it and continue scanning from one char past."""
    pos = start
    n = len(text)
    while pos < n:
        r = find_balanced_paren_block(text, pos)
        if r is None:
            # Either no more openers, or the next opener is unbalanced.
            # Try advancing past the next opener so we keep scanning.
            j = pos
            while j < n and text[j] not in _OPEN_PARENS:
                j += 1
            if j >= n:
                return
            pos = j + 1
            continue
        yield r
        pos = r[1]


def find_first_verdict_paren(entry: str, verdict_words):
    """Within `entry`, find the first balanced-paren block whose inner text
    starts with one of `verdict_words`. Returns (block_start, block_end, inner)
    or None.
    """
    pos = 0
    while True:
        result = find_balanced_paren_block(entry, pos)
        if result is None:
            return None
        bs, be, inner = result
        stripped = inner.lstrip()
        if any(stripped.startswith(w) for w in verdict_words):
            return result
        pos = be


# ── Mustalah vocabulary ──
#
# Comprehensive list of hadith-verdict terms Albani uses. Per the user's
# direction, ALL mustalah terms must be captured verbatim. The list is the
# UNION of words observed across the seven graded books; per-book scripts
# share this list rather than each maintaining its own.
#
# Order matters for prefix-matching (longer phrases first so "حسن صحيح" wins
# over "حسن"). Each entry pairs the term with a coarse normalized bucket used
# for color-coding in the UI; the verbatim Arabic term is kept in the row's
# `grade` field for display.

MUSTALAH_VERDICTS = [
    # tier modifiers go first
    ("حسن صحيح",          "hasan"),   # hasan-sahih, often used by Tirmidhi
    ("صحيح لغيره",        "sahih"),
    ("حسن لغيره",         "hasan"),
    ("صحيح الإسناد",      "sahih"),
    ("حسن الإسناد",       "hasan"),
    ("ضعيف الإسناد",      "daif"),
    ("ضعيف جدا",          "daif"),
    ("ضعيف الحديث",       "daif"),
    ("لين الحديث",        "daif"),
    ("متروك الحديث",      "daif"),
    ("منكر",              "daif"),
    ("شاذ",               "daif"),
    ("صحيح",              "sahih"),
    ("حسن",               "hasan"),
    ("ضعيف",              "daif"),
    ("موضوع",             "mawdu"),
    ("باطل",              "mawdu"),
    # status / chain-quality terms (don't translate to a tier on their own)
    ("متروك",             "daif"),
    ("كذاب",              "daif"),
    ("وضاع",              "mawdu"),
    ("مقطوع",             "other"),
    ("موقوف",             "other"),
    ("مرسل",              "other"),
    ("منقطع",             "other"),
    ("معلول",             "other"),
    ("معضل",              "other"),
    ("غريب",              "other"),
    ("لا أصل له",         "mawdu"),
    ("لا يصح",            "daif"),
]

_VERDICT_PREFIX_LIST = [w for w, _ in MUSTALAH_VERDICTS]
_VERDICT_BUCKET = {w: b for w, b in MUSTALAH_VERDICTS}


def _is_footnote_or_ref(inner: str) -> bool:
    """Skip parens that aren't verdicts: footnote markers (^1), page refs
    (8/212), ellipses (...), and tiny tags."""
    s = inner.strip()
    if len(s) < 2:
        return True
    # Footnote / numeric only
    if re.fullmatch(r"[\^\d٠-٩\s\.\,]+", s):
        return True
    # Page reference like 8/212 or ١/٢
    if re.fullmatch(r"[\d٠-٩]+\s*[/\\]\s*[\d٠-٩]+(?:\s*[\d٠-٩]+)?", s):
        return True
    return False


def find_albani_verdict(entry: str):
    """Within an entry block, return the LAST balanced-paren block whose
    stripped inner text starts with a known mustalah verdict word.

    Returns (start, end, inner_verbatim, normalized_bucket) or None.

    Why "last": Albani's discussion-style entries quote other scholars first
    and conclude with his own verdict, which always lives in the final paren.
    Tabular entries have only one paren (the verdict), so first-vs-last is
    irrelevant there.

    Uses iter_paren_blocks so unbalanced parens don't abort the scan.
    """
    candidates = []
    for bs, be, inner in iter_paren_blocks(entry):
        if _is_footnote_or_ref(inner):
            continue
        stripped = inner.lstrip()
        for prefix in _VERDICT_PREFIX_LIST:
            if stripped.startswith(prefix):
                candidates.append((bs, be, inner, _VERDICT_BUCKET[prefix]))
                break
    return candidates[-1] if candidates else None


def output_path_for_book(book_id: int) -> str:
    return os.path.join(DATA_DIR, f"grading_book_{book_id}.json")


def write_rows(book_id: int, rows: list) -> str:
    """Write extracted rows to data/grading_book_<id>.json atomically."""
    out = output_path_for_book(book_id)
    tmp = out + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(rows, f, ensure_ascii=False, indent=2)
    os.replace(tmp, out)
    return out
