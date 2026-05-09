#!/usr/bin/env python3
"""
Per-book extractor — Turath book 5914, Albani's "ضعيف أبي داود - الأم".
Maps to our collection_id=3 (Sunan Abi Dawud).

Same al-Umm prose-discussion format as book 25881 (Sahih Abu Dawud al-Umm):
each entry is numbered, then a multi-paragraph isnad/takhrij analysis, with
Albani's own concluding verdict in a `(قلت: ...)` parenthetical.

Output: data/grading_book_5914.json
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

BOOK_ID = 5914
SLUG = "daif_sunan_abi_dawud"
COLLECTION_ID = 3
SCHOLAR_KEY = "albani"
SCHOLAR_AR = "الألباني"

# Some daif al-Umm entries use `<num>/م` or `<num>/ك` suffixes for sub-numbered
# variants. We accept the optional suffix and capture only the leading number.
ENTRY_HEAD_RE = re.compile(
    r"^[ \t]*([\d٠-٩]+)(?:\s*/\s*[مكأبج])?\s*-\s+(?:عن|حدثنا|أخبرنا|قال|روى)",
    re.MULTILINE,
)

QULT_PREFIX = re.compile(r"^\s*قلت\s*:")

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

    qult_verdicts = []
    for bs, be, inner in iter_paren_blocks(full):
        if not QULT_PREFIX.match(inner):
            continue
        vm = _VERDICT_WORDS_RE.search(inner[:250])
        if vm is None:
            continue
        verdict_term = vm.group(1)
        bucket = _VERDICT_BUCKET[verdict_term]
        qult_verdicts.append((bs, be, inner, verdict_term, bucket))

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
