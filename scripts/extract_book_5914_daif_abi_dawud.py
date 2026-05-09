#!/usr/bin/env python3
"""
Per-book extractor — Turath book 5914, Albani's "ضعيف أبي داود - الأم".
Maps to our collection_id=3 (Sunan Abi Dawud).

Same al-Umm prose-discussion format as book 25881; same per-entry strategy
with قلت-anchor first then a verdict-paren fallback.

Output: data/grading_book_5914.json
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

# Daif al-Umm sometimes uses `<num>/م` or `<num>/ك` sub-numbering — capture
# only the leading number.
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


def find_qult_verdicts(entry_text: str):
    """Return ALL قلت-verdict blocks in the entry (one row per chain Albani
    discusses; the schema allows multiple rows per hadith)."""
    candidates = []
    for bs, be, inner in iter_paren_blocks(entry_text):
        if not QULT_PREFIX.match(inner):
            continue
        vm = _VERDICT_WORDS_RE.search(inner[:250])
        if vm is None:
            continue
        verdict_term = vm.group(1)
        bucket = _VERDICT_BUCKET[verdict_term]
        candidates.append((bs, be, inner, verdict_term, bucket))
    return candidates


def main():
    pages = load_pages(SLUG)
    if pages is None:
        print(f"ERROR: data/{SLUG}_pages.json not found.", file=sys.stderr)
        sys.exit(1)

    full, offsets = concatenate_pages(pages)
    headers = list(ENTRY_HEAD_RE.finditer(full))
    print(f"Book {BOOK_ID} ({SLUG}): {len(pages)} pages, "
          f"{len(headers)} entry headers detected")

    rows = []
    n_qult = 0
    n_fallback = 0
    n_no_verdict = 0
    n_unique_hadiths = set()
    not_in_corpus_count = 0

    for i, h in enumerate(headers):
        try:
            hadith_number = int(normalize_digits(h.group(1)))
        except ValueError:
            continue

        start = h.end()
        end = headers[i + 1].start() if i + 1 < len(headers) else len(full)
        entry = full[start:end]

        verdicts = find_qult_verdicts(entry)
        used_path = "qult"
        if not verdicts:
            v2 = find_albani_verdict(entry)
            if v2 is not None:
                bs, be, inner, bucket = v2
                verdict_term = inner.split()[0] if inner.strip() else ""
                verdicts = [(bs, be, inner, verdict_term, bucket)]
                used_path = "fallback"

        if not verdicts:
            n_no_verdict += 1
            continue

        n_unique_hadiths.add(hadith_number)
        for bs, be, inner, verdict_term, bucket in verdicts:
            if used_path == "qult":
                n_qult += 1
            else:
                n_fallback += 1

            if not in_corpus(COLLECTION_ID, hadith_number):
                not_in_corpus_count += 1

            global_off = start + bs
            page_idx = page_for_offset(offsets, global_off)

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
    print(f"    via qult-anchor          : {n_qult}")
    print(f"    via verdict-paren fallback: {n_fallback}")
    print(f"  unique hadiths covered     : {len(n_unique_hadiths)}")
    print(f"  entries with no verdict    : {n_no_verdict}")
    print(f"  rows w/ hadith# not in corpus: {not_in_corpus_count}")
    out = write_rows(BOOK_ID, rows)
    print(f"\nWrote {len(rows)} rows to {out}")


if __name__ == "__main__":
    main()
