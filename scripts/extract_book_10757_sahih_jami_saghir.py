#!/usr/bin/env python3
"""
Per-book extractor — Turath book 10757, Albani's "صحيح الجامع الصغير وزيادته".

This book is NOT a Sunan grading book (it grades Suyuti's Jami al-Saghir
collection by his own numbering). What we want from it is Albani's
cross-references back to the four Sunan: each entry ends with a tag like

    صحيح أبي داود ٦٨٣
    صحيح الترمذي ٢٤١٧
    صحيح النسائي ١٤٧
    صحيح ابن ماجه ٨٠٤

These cross-refs let us populate Albani's verdict on a Sunan hadith identified
by `(parsed_number, collection_id)` even when no standalone Albani-Sunan
extractor is available (notably for Sahih Tirmidhi and Sahih/Daif Ibn Majah).

Output: data/grading_book_10757.json
"""

import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _grading_common import (  # noqa: E402
    concatenate_pages,
    in_corpus,
    load_pages,
    normalize_digits,
    page_for_offset,
    strip_diacritics,
    strip_html,
    write_rows,
)

BOOK_ID = 10757
SLUG = "sahih_jami_saghir"
SCHOLAR_KEY = "albani"
SCHOLAR_AR = "الألباني"

# Cross-reference: <verdict-word> <sunan-name> <number>.
# Note: this book is "Sahih al-Jami" so most refs are `صحيح <sunan> N`, but
# some entries reference daif copies too (`ضعيف <sunan> N`) when discussing
# weak narrations. We accept both verdicts.
REF_RE = re.compile(
    r"(صحيح|ضعيف)\s+"
    r"(أبي\s+داود|أبي\s+داوود|الترمذي|النسائي|ابن\s+ماجه|ابن\s+ماجة)"
    r"\s+([\d٠-٩]+)"
)

SUNAN_TO_COLLECTION = {
    "أبي داود": 3,
    "أبي داوود": 3,
    "الترمذي": 4,
    "النسائي": 5,
    "ابن ماجه": 6,
    "ابن ماجة": 6,
}

VERDICT_TO_BUCKET = {
    "صحيح": "sahih",
    "ضعيف": "daif",
}


def normalize_sunan_key(raw):
    return re.sub(r"\s+", " ", raw).strip()


def main():
    pages = load_pages(SLUG)
    if pages is None:
        print(f"ERROR: data/{SLUG}_pages.json not found.", file=sys.stderr)
        sys.exit(1)

    rows = []
    refs_total = 0
    refs_resolved = 0
    not_in_corpus_count = 0
    seen = set()  # (collection_id, hadith_number) — dedup per book

    for p in pages:
        page_index = (p.get("page_id") or 1) - 1
        text = strip_diacritics(strip_html(p.get("text", "")))
        for m in REF_RE.finditer(text):
            refs_total += 1
            verdict_raw = m.group(1)
            sunan_raw = normalize_sunan_key(m.group(2))
            num_str = normalize_digits(m.group(3))
            collection_id = SUNAN_TO_COLLECTION.get(sunan_raw)
            bucket = VERDICT_TO_BUCKET.get(verdict_raw)
            if collection_id is None or bucket is None:
                continue
            try:
                hadith_number = int(num_str)
            except ValueError:
                continue
            key = (collection_id, hadith_number)
            if key in seen:
                continue
            seen.add(key)
            refs_resolved += 1
            if not in_corpus(collection_id, hadith_number):
                not_in_corpus_count += 1
            rows.append({
                "hadith_number": hadith_number,
                "collection_id": collection_id,
                "scholar_key": SCHOLAR_KEY,
                "scholar_ar": SCHOLAR_AR,
                "grade": f"{verdict_raw} {sunan_raw} {num_str}",
                "grade_normalized": bucket,
                "source_book_id": BOOK_ID,
                "source_page_index": page_index,
                "raw_text": m.group(0)[:120],
                "notes": "cross-reference from Sahih al-Jami al-Saghir",
            })

    print(f"Book {BOOK_ID} ({SLUG}): {len(pages)} pages")
    print(f"  cross-references total     : {refs_total}")
    print(f"  unique resolved rows       : {len(rows)}")
    print(f"  rows w/ hadith# not in corpus: {not_in_corpus_count}")
    out = write_rows(BOOK_ID, rows)
    print(f"\nWrote {len(rows)} rows to {out}")


if __name__ == "__main__":
    main()
