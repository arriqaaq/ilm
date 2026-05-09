#!/usr/bin/env python3
"""
Per-book extractor — Turath book 1216, Albani's "ضعيف سنن الترمذي".
Maps to our collection_id=4 (Jami at-Tirmidhi).

Format (verified by sampling pages 50, 60, 100, 200, 300, 400):
    <baab_idx> - <tirmidhi_hadith#> [optional (^N)] (chain word) ...
    (<verdict>)[ - <cross-reference>]

This Turath copy of book 1216 has aggressive line-collapsing — entry headers
frequently appear inline after the previous entry's verdict on the same
physical line. So instead of iterating entry-by-entry (which works for the
cleanly-broken 1147 / 1148 books), we iterate VERDICT-by-VERDICT: scan the
text for balanced-paren blocks whose content starts with a mustalah verdict,
and for each, walk back ≤2000 chars to the nearest `<baab>-<hadith> <chain>`
header.

Output: data/grading_book_1216.json
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

BOOK_ID = 1216
SLUG = "daif_sunan_tirmidhi"
COLLECTION_ID = 4
SCHOLAR_KEY = "albani"
SCHOLAR_AR = "الألباني"

# Edition-specific OCR typo (`ضععيف` for `ضعيف`).
_LOCAL_TYPOS = [("ضععيف", "daif")]
for term, bucket in _LOCAL_TYPOS:
    if (term, bucket) not in MUSTALAH_VERDICTS:
        MUSTALAH_VERDICTS.insert(0, (term, bucket))
import _grading_common  # noqa: E402
_grading_common._VERDICT_PREFIX_LIST = [w for w, _ in MUSTALAH_VERDICTS]
_grading_common._VERDICT_BUCKET = {w: b for w, b in MUSTALAH_VERDICTS}

# Walk-back: nearest `<baab> - <hadith>` followed by chain word. May appear
# anywhere (no line anchor) since entries collapse inline in this edition.
ENTRY_HEAD_RE = re.compile(
    r"(?<![\d٠-٩])([\d٠-٩]+)\s*-\s*([\d٠-٩]+)\s*"
    r"(?:\(\^?[\d٠-٩]+\)\s*)?"
    r"(?:عن|حدثنا|أخبرنا|قال)"
)

WALKBACK = 2000


def main():
    pages = load_pages(SLUG)
    if pages is None:
        print(f"ERROR: data/{SLUG}_pages.json not found.", file=sys.stderr)
        sys.exit(1)

    full, offsets = concatenate_pages(pages)
    print(f"Book {BOOK_ID} ({SLUG}): {len(pages)} pages, full_text len={len(full)}")

    # Find all verdict-paren blocks (iter_paren_blocks skips unbalanced parens
    # rather than aborting — important because this Turath copy uses `»` as an
    # alternate close paren in 492 places).
    verdict_blocks = []
    for bs, be, inner in iter_paren_blocks(full):
        stripped = inner.lstrip()
        for prefix in _grading_common._VERDICT_PREFIX_LIST:
            if stripped.startswith(prefix):
                verdict_blocks.append((bs, be, inner, _grading_common._VERDICT_BUCKET[prefix]))
                break

    print(f"  verdict-paren blocks       : {len(verdict_blocks)}")

    rows, no_anchor = [], 0
    not_in_corpus = []
    seen = set()  # (hadith_number, source_page_index) — dedup duplicates

    for bs, be, inner, bucket in verdict_blocks:
        # Walk back at most WALKBACK chars to find the latest entry header.
        win_start = max(0, bs - WALKBACK)
        window = full[win_start:bs]
        candidates = list(ENTRY_HEAD_RE.finditer(window))
        if not candidates:
            no_anchor += 1
            continue
        last = candidates[-1]
        try:
            hadith_number = int(normalize_digits(last.group(2)))
        except ValueError:
            continue

        page_idx = page_for_offset(offsets, bs)
        key = (hadith_number, page_idx)
        if key in seen:
            continue
        seen.add(key)

        if not in_corpus(COLLECTION_ID, hadith_number):
            not_in_corpus.append(hadith_number)

        rows.append({
            "hadith_number": hadith_number,
            "collection_id": COLLECTION_ID,
            "scholar_key": SCHOLAR_KEY,
            "scholar_ar": SCHOLAR_AR,
            "grade": inner.strip(),
            "grade_normalized": bucket,
            "source_book_id": BOOK_ID,
            "source_page_index": page_idx,
            "raw_text": ("(" + inner.strip() + ")")[:200],
        })

    print(f"  rows extracted             : {len(rows)}")
    print(f"  verdicts with no anchor    : {no_anchor}")
    print(f"  rows w/ hadith# not in corpus (will be skipped at ingest): {len(not_in_corpus)}")
    out = write_rows(BOOK_ID, rows)
    print(f"\nWrote {len(rows)} rows to {out}")


if __name__ == "__main__":
    main()
