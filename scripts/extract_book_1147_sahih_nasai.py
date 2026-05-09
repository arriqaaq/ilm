#!/usr/bin/env python3
"""
Per-book extractor — Turath book 1147, Albani's "صحيح سنن النسائي".
Maps to our collection_id=5 (Sunan an-Nasa'i).

Format: tabular numbered entries
    <hadith_number> - (عن|حدثنا|أخبرنا|قال) <chain> ... <matn>
    (<verdict>)[ - <cross-reference>]

Verdict extraction is delegated to _grading_common.find_albani_verdict, which
balances parens (handles nested footnote refs) and recognizes the full
mustalah vocabulary. The verdict's verbatim text is stored as-is.

Output: data/grading_book_1147.json
"""

import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _grading_common import (  # noqa: E402
    MUSTALAH_VERDICTS,
    concatenate_pages,
    find_albani_verdict,
    in_corpus,
    load_pages,
    normalize_digits,
    page_for_offset,
    write_rows,
)

# Edition-specific OCR typos for `صحيح` observed in this Turath copy of book
# 1147 (entries #337 and #4043). Inject them into the global mustalah list at
# import time so the shared verdict finder treats them as `صحيح`.
_LOCAL_TYPOS = [("صجيح", "sahih"), ("صحهح", "sahih")]
for term, bucket in _LOCAL_TYPOS:
    if (term, bucket) not in MUSTALAH_VERDICTS:
        MUSTALAH_VERDICTS.append((term, bucket))
# Reset the precomputed lookup tables in _grading_common after mutation.
import _grading_common  # noqa: E402
_grading_common._VERDICT_PREFIX_LIST = [w for w, _ in MUSTALAH_VERDICTS]
_grading_common._VERDICT_BUCKET = {w: b for w, b in MUSTALAH_VERDICTS}

BOOK_ID = 1147
SLUG = "sahih_sunan_nasai"
COLLECTION_ID = 5
SCHOLAR_KEY = "albani"
SCHOLAR_AR = "الألباني"

# Entry header: line beginning with `<hadith#> - <chain word>`.
ENTRY_HEAD_RE = re.compile(
    r"^[ \t]*([\d٠-٩]+)\s*-\s+(?:عن|حدثنا|أخبرنا|قال)",
    re.MULTILINE,
)


def main():
    pages = load_pages(SLUG)
    if pages is None:
        print(f"ERROR: data/{SLUG}_pages.json not found.", file=sys.stderr)
        sys.exit(1)

    full, offsets = concatenate_pages(pages)
    headers = list(ENTRY_HEAD_RE.finditer(full))
    print(f"Book {BOOK_ID} ({SLUG}): {len(pages)} pages, {len(headers)} entry headers")

    rows, no_verdict, not_in_corpus = [], [], []
    seen = set()

    for i, h in enumerate(headers):
        try:
            hadith_number = int(normalize_digits(h.group(1)))
        except ValueError:
            continue

        start = h.end()
        end = headers[i + 1].start() if i + 1 < len(headers) else len(full)
        entry = full[start:end]

        v = find_albani_verdict(entry)
        if v is None:
            no_verdict.append((hadith_number, page_for_offset(offsets, h.start())))
            continue
        bs, be, inner, bucket = v

        if hadith_number in seen:
            continue
        seen.add(hadith_number)

        if not in_corpus(COLLECTION_ID, hadith_number):
            not_in_corpus.append(hadith_number)

        page_idx = page_for_offset(offsets, start + bs)
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
    print(f"  entries with no verdict    : {len(no_verdict)}")
    print(f"  rows w/ hadith# not in corpus (will be skipped at ingest): {len(not_in_corpus)}")
    out = write_rows(BOOK_ID, rows)
    print(f"\nWrote {len(rows)} rows to {out}")


if __name__ == "__main__":
    main()
