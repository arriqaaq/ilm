#!/usr/bin/env python3
"""
Per-book extractor — Turath book 1663, Albani's "ضعيف الجامع الصغير وزيادته".

Format note (verified by sampling pages 50, 100, 200, 500):
  This book uses Suyuti's Jami al-Saghir numbering, NOT Sunan numbering.
  Each entry has the form:
      <suyuti#> - <matn>
      (<source-abbreviation-codes>) عن <sahabi>.
      ─────
      (<verdict>)
  where source codes are short letters like (حم) for Ahmad, (د هق) for Abu
  Dawud + Bayhaqi, (طس) for Tabarani al-Awsat — NOT explicit Sunan
  cross-references.

Therefore book 1663 produces ZERO extractable verdict rows: there's no way to
map Suyuti's numbering to our Sunan-keyed hadith corpus without a fuzzy matn
matcher, which is deferred to v2. The book is still ingested into book / book_page
so the in-app reader can open it. The extractor here just writes an empty
JSON file for pipeline-uniformity.

Output: data/grading_book_1663.json (empty array)
"""

import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _grading_common import (  # noqa: E402
    in_corpus,
    load_pages,
    normalize_digits,
    strip_diacritics,
    strip_html,
    write_rows,
)

BOOK_ID = 1663
SLUG = "daif_jami_saghir"
SCHOLAR_KEY = "albani"
SCHOLAR_AR = "الألباني"

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

VERDICT_TO_BUCKET = {"صحيح": "sahih", "ضعيف": "daif"}


def main():
    pages = load_pages(SLUG)
    if pages is None:
        print(f"ERROR: data/{SLUG}_pages.json not found.", file=sys.stderr)
        sys.exit(1)

    rows = []
    refs_total = 0
    not_in_corpus_count = 0
    seen = set()

    for p in pages:
        page_index = (p.get("page_id") or 1) - 1
        text = strip_diacritics(strip_html(p.get("text", "")))
        for m in REF_RE.finditer(text):
            refs_total += 1
            verdict_raw = m.group(1)
            sunan_raw = re.sub(r"\s+", " ", m.group(2)).strip()
            try:
                hadith_number = int(normalize_digits(m.group(3)))
            except ValueError:
                continue
            collection_id = SUNAN_TO_COLLECTION.get(sunan_raw)
            bucket = VERDICT_TO_BUCKET.get(verdict_raw)
            if collection_id is None or bucket is None:
                continue
            key = (collection_id, hadith_number)
            if key in seen:
                continue
            seen.add(key)
            if not in_corpus(collection_id, hadith_number):
                not_in_corpus_count += 1
            rows.append({
                "hadith_number": hadith_number,
                "collection_id": collection_id,
                "scholar_key": SCHOLAR_KEY,
                "scholar_ar": SCHOLAR_AR,
                "grade": f"{verdict_raw} {sunan_raw} {m.group(3)}",
                "grade_normalized": bucket,
                "source_book_id": BOOK_ID,
                "source_page_index": page_index,
                "raw_text": m.group(0)[:120],
                "notes": "cross-reference from Daif al-Jami al-Saghir",
            })

    print(f"Book {BOOK_ID} ({SLUG}): {len(pages)} pages")
    print(f"  cross-references total     : {refs_total}")
    print(f"  unique resolved rows       : {len(rows)}")
    print(f"  rows w/ hadith# not in corpus: {not_in_corpus_count}")
    out = write_rows(BOOK_ID, rows)
    print(f"\nWrote {len(rows)} rows to {out}")


if __name__ == "__main__":
    main()
