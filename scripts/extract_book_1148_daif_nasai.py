#!/usr/bin/env python3
"""
Per-book extractor — Turath book 1148, Albani's "ضعيف سنن النسائي".
Maps to our collection_id=5 (Sunan an-Nasa'i).

Format differs from book 1147: each entry has TWO numbers separated by a
dash, the SECOND being the Nasa'i hadith number (the first is Albani's
sequential daif-index, ignored).

    <baab_idx> - <nasai_hadith#> [optional (^N)] (عن|حدثنا|أخبرنا|قال) <chain>
    (<verdict>)[ - <cross-reference>]

Verdict extraction goes through _grading_common.find_albani_verdict, same
shared mustalah vocabulary as the other books.

Output: data/grading_book_1148.json
"""

import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _grading_common import (  # noqa: E402
    concatenate_pages,
    find_albani_verdict,
    in_corpus,
    load_pages,
    normalize_digits,
    page_for_offset,
    write_rows,
)

BOOK_ID = 1148
SLUG = "daif_sunan_nasai"
COLLECTION_ID = 5
SCHOLAR_KEY = "albani"
SCHOLAR_AR = "الألباني"

# `<baab> - <hadith>  [(^N)] (chain word)`
ENTRY_HEAD_RE = re.compile(
    r"^[ \t]*([\d٠-٩]+)\s*-\s*([\d٠-٩]+)\s*(?:\(\^?[\d٠-٩]+\)\s*)?(?:عن|حدثنا|أخبرنا|قال)",
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
            hadith_number = int(normalize_digits(h.group(2)))  # second number
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
