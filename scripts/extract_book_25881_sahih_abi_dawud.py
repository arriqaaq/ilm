#!/usr/bin/env python3
"""
Per-book extractor — Turath book 25881, Albani's "صحيح سنن أبي داود ط غراس".
Maps to our collection_id=3 (Sunan Abi Dawud).

Format (verified by sampling pages 200, 500, 1000):
  Full prose-discussion edition. Each entry is numbered:
      <hadith#> - عن <sahabi> ... <matn>
  Then comes a multi-paragraph isnad/takhrij discussion that quotes other
  scholars (al-Hakim, Dhahabi, Ibn Hibban, ...). Albani's OWN verdict appears
  in a parenthetical that begins with `قلت:` (his first-person voice marker):
      (قلت: إسناده صحيح على شرط البخاري ... )

Strategy:
  Iterate balanced-paren blocks across the whole book. For each block whose
  stripped inner text starts with `قلت\\s*:`, extract the verdict by scanning
  the first ~150 chars of the inner text for a mustalah verdict word
  (إسناده X / حديث X / ...). Walk back ≤4000 chars to the nearest entry header
  to attribute the verdict to a hadith number.

This handles Albani's multiple verdicts within a single entry (e.g.,
"chain A is sahih, chain B is daif") naturally — each `(قلت:...)` block
becomes its own row.

Output: data/grading_book_25881.json
"""

import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _grading_common import (  # noqa: E402
    MUSTALAH_VERDICTS,
    concatenate_pages,
    in_corpus,
    iter_paren_blocks,
    load_pages,
    normalize_digits,
    page_for_offset,
    write_rows,
)

BOOK_ID = 25881
SLUG = "sahih_sunan_abi_dawud"
COLLECTION_ID = 3
SCHOLAR_KEY = "albani"
SCHOLAR_AR = "الألباني"

# Entry head: `<hadith#> - <chain word>` at start of line.
ENTRY_HEAD_RE = re.compile(
    r"^[ \t]*([\d٠-٩]+)\s*-\s+(?:عن|حدثنا|أخبرنا|قال|روى)",
    re.MULTILINE,
)

# Mustalah verdict word + optional cross-ref text inside the قلت paren.
QULT_PREFIX = re.compile(r"^\s*قلت\s*:")

# Once we know we're inside a قلت block, look for the actual verdict word.
# Use the shared mustalah vocabulary, but allow it anywhere in the first
# 200 chars (Albani prefixes his verdict with explanatory context like
# "وهذا إسناده حسن", "والحديث صحيح", etc.).
_VERDICT_WORDS_RE = re.compile(
    r"(?<![ء-ي])("
    + "|".join(re.escape(w) for w, _ in MUSTALAH_VERDICTS)
    + r")(?![ء-ي])"
)
_VERDICT_BUCKET = {w: b for w, b in MUSTALAH_VERDICTS}

WALKBACK = 4000


def main():
    pages = load_pages(SLUG)
    if pages is None:
        print(f"ERROR: data/{SLUG}_pages.json not found.", file=sys.stderr)
        sys.exit(1)

    full, offsets = concatenate_pages(pages)
    print(f"Book {BOOK_ID} ({SLUG}): {len(pages)} pages, full_text len={len(full)}")

    # Find every قلت: paren block + its verdict word.
    qult_verdicts = []
    for bs, be, inner in iter_paren_blocks(full):
        if not QULT_PREFIX.match(inner):
            continue
        # Search for a verdict word in the first 250 chars.
        head = inner[:250]
        vm = _VERDICT_WORDS_RE.search(head)
        if vm is None:
            continue  # qult comment without a verdict (just a clarifying note)
        verdict_term = vm.group(1)
        bucket = _VERDICT_BUCKET[verdict_term]
        qult_verdicts.append((bs, be, inner, verdict_term, bucket))

    print(f"  قلت paren blocks total     : (counted via iteration)")
    print(f"  قلت blocks with verdict    : {len(qult_verdicts)}")

    rows, no_anchor = [], 0
    not_in_corpus_count = 0

    for bs, be, inner, verdict_term, bucket in qult_verdicts:
        win_start = max(0, bs - WALKBACK)
        window = full[win_start:bs]
        candidates = list(ENTRY_HEAD_RE.finditer(window))
        if not candidates:
            no_anchor += 1
            continue
        last = candidates[-1]
        try:
            hadith_number = int(normalize_digits(last.group(1)))
        except ValueError:
            continue

        page_idx = page_for_offset(offsets, bs)

        if not in_corpus(COLLECTION_ID, hadith_number):
            not_in_corpus_count += 1

        rows.append({
            "hadith_number": hadith_number,
            "collection_id": COLLECTION_ID,
            "scholar_key": SCHOLAR_KEY,
            "scholar_ar": SCHOLAR_AR,
            "grade": inner.strip()[:300],
            "grade_normalized": bucket,
            "source_book_id": BOOK_ID,
            "source_page_index": page_idx,
            "raw_text": ("(" + inner.strip() + ")")[:300],
        })

    print(f"  rows extracted             : {len(rows)}")
    print(f"  verdicts with no anchor    : {no_anchor}")
    print(f"  rows w/ hadith# not in corpus: {not_in_corpus_count}")
    out = write_rows(BOOK_ID, rows)
    print(f"\nWrote {len(rows)} rows to {out}")


if __name__ == "__main__":
    main()
