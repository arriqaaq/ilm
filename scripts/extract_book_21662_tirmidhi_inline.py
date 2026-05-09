#!/usr/bin/env python3
"""
Per-book extractor — Turath book 21662, Tuhfat al-Ahwadhi (al-Mubarakfuri).

Tuhfat is the canonical sharh of Jami at-Tirmidhi. It reproduces Tirmidhi's
full text — including his own inline gradings — embedded in the commentary.
We use it as the vehicle for extracting *Tirmidhi's own verdicts* on his
hadiths, which Albani's standalone "Sahih Sunan al-Tirmidhi" doesn't exist
for in Turath.

The project already maintains a (hadith_number → page_index) mapping at
`data/tuhfat_ahwadhi_hadith_mapping.json` — 3,956 entries covering every
Tirmidhi hadith. That gives us a clean anchor: for each hadith, look at the
3 pages starting from its mapped page, find Tirmidhi's verdict in the
commentary, and emit a row.

Distinct from Albani's grading rows: this script emits `scholar_key="tirmidhi"`
so the UI clearly shows it as Tirmidhi's own verdict (alongside any Albani
verdict that may exist).

Output: data/grading_book_21662_tirmidhi_inline.json
"""

import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _grading_common import (  # noqa: E402
    DATA_DIR,
    in_corpus,
    strip_diacritics,
    strip_html,
    write_rows,
)

BOOK_ID = 21662
COLLECTION_ID = 4
SCHOLAR_KEY = "tirmidhi"
SCHOLAR_AR = "الترمذي"

PAGES_PATH = os.path.join(DATA_DIR, "tuhfat_ahwadhi_pages.json")
MAPPING_PATH = os.path.join(DATA_DIR, "tuhfat_ahwadhi_hadith_mapping.json")

# Tirmidhi's verdicts as quoted in Tuhfat. Two flavors:
#  1. The sharh wrapper — `قوله (هذا حديث X)` — al-Mubarakfuri quotes
#     Tirmidhi's verdict to comment on it. Highest-confidence match.
#  2. Bare `هذا حديث X` (or `قال أبو عيسى ... حسن صحيح`) — Tirmidhi's voice
#     reproduced in the matn block before the commentary.
QULA_RE = re.compile(
    r"قوله\s*\(?\s*هذا\s+حديث\s+"
    r"(حسن\s+صحيح\s+غريب|حسن\s+غريب\s+صحيح|حسن\s+صحيح|حسن\s+غريب|"
    r"صحيح\s+غريب|صحيح|حسن|غريب)"
)
BARE_RE = re.compile(
    r"(?:قال\s+أبو\s+عيسى[^\n]{0,80})?(?:^|[\.\n,])\s*هذا\s+حديث\s+"
    r"(حسن\s+صحيح\s+غريب|حسن\s+غريب\s+صحيح|حسن\s+صحيح|حسن\s+غريب|"
    r"صحيح\s+غريب|صحيح|حسن|غريب)"
)

# Priority-ordered grade normalization. First match wins.
GRADE_RULES = [
    (re.compile(r"حسن\s+صحيح\s+غريب|حسن\s+غريب\s+صحيح"), "hasan"),
    (re.compile(r"حسن\s+صحيح"), "hasan"),
    (re.compile(r"صحيح\s+غريب"), "sahih"),
    (re.compile(r"^صحيح$"), "sahih"),
    (re.compile(r"حسن\s+غريب"), "other"),
    (re.compile(r"^حسن$"), "hasan"),
    (re.compile(r"^غريب$"), "other"),
]


def normalize_grade(phrase: str) -> str:
    s = phrase.strip()
    for rx, norm in GRADE_RULES:
        if rx.search(s):
            return norm
    return "other"


def main():
    if not os.path.exists(PAGES_PATH):
        print(f"ERROR: {PAGES_PATH} not present.", file=sys.stderr)
        sys.exit(1)
    if not os.path.exists(MAPPING_PATH):
        print(f"ERROR: {MAPPING_PATH} not present.", file=sys.stderr)
        sys.exit(1)

    with open(PAGES_PATH, "r", encoding="utf-8") as f:
        pages = json.load(f)
    with open(MAPPING_PATH, "r", encoding="utf-8") as f:
        mapping = json.load(f)

    # Index pages by 0-based page_index (the JSON's page_id is 1-based).
    by_idx = {}
    for p in pages:
        pid = p.get("page_id")
        if isinstance(pid, int):
            by_idx[pid - 1] = strip_diacritics(strip_html(p.get("text", "")))

    # Detect placeholder mappings: if many hadiths share the same page_index
    # (>5), the mapping for that page is unreliable — mass-mapped to a
    # sentinel rather than a real per-hadith location. Audit found ~1,223
    # hadiths sharing page 4869 alone, plus a long tail of 8-9 hadiths per
    # page near the end of the mapping. Skip all of them rather than emit
    # garbage rows.
    from collections import Counter
    page_counts = Counter()
    for info in mapping.values():
        if isinstance(info, dict) and info.get("page_index") is not None:
            page_counts[info["page_index"]] += 1
    PLACEHOLDER_PAGES = {pi for pi, c in page_counts.items() if c > 5}

    print(f"Book {BOOK_ID} (tuhfat_ahwadhi): {len(pages)} pages, "
          f"{len(mapping)} mapping entries; "
          f"skipping {len(PLACEHOLDER_PAGES)} placeholder pages "
          f"(holding {sum(page_counts[pi] for pi in PLACEHOLDER_PAGES)} bogus hadith mappings)")

    # Group hadiths by their mapped starting page so we can do a positional
    # alignment when several hadiths share a page (Tuhfat compresses the
    # commentary for consecutive short hadiths on one page).
    from collections import defaultdict
    page_to_hadiths = defaultdict(list)
    for k, info in mapping.items():
        try:
            hadith_number = int(k)
        except (TypeError, ValueError):
            continue
        if not isinstance(info, dict):
            continue
        page_index = info.get("page_index")
        if page_index is None:
            continue
        if page_index in PLACEHOLDER_PAGES:
            continue
        page_to_hadiths[page_index].append(hadith_number)
    for pi in page_to_hadiths:
        page_to_hadiths[pi].sort()  # ensure ascending hadith order

    rows = []
    n_attempted = 0
    n_resolved = 0
    n_no_verdict = 0
    n_missing_page = 0
    n_not_in_corpus = 0
    n_count_mismatch = 0
    n_placeholder = sum(1 for info in mapping.values()
                        if isinstance(info, dict)
                        and info.get("page_index") in PLACEHOLDER_PAGES)

    for page_index, hadiths in page_to_hadiths.items():
        n_attempted += len(hadiths)

        # Build a 3-page window (matn often spills onto the next page).
        chunks = []
        for off in (0, 1, 2):
            txt = by_idx.get(page_index + off)
            if txt:
                chunks.append(txt)
        if not chunks:
            n_missing_page += len(hadiths)
            continue
        block = "\n".join(chunks)
        block = block[:6000 + 2000 * (len(hadiths) - 1)]  # widen for many-hadith pages

        # Find ALL verdict matches on the page in order.
        all_matches = []
        for rx in (QULA_RE, BARE_RE):
            for m in rx.finditer(block):
                all_matches.append((m.start(), m.group(1).strip()))
        # Dedup by offset and sort
        all_matches = sorted(set(all_matches))

        # When the number of verdict matches equals the number of hadiths on
        # this page, align them positionally (hadith[i] → match[i]). When
        # they don't match exactly, only emit rows for hadiths up to
        # min(len(hadiths), len(all_matches)) — we'd rather miss than
        # mis-attribute. For pages with one hadith and one match, this is
        # the same as the old behaviour.
        usable = min(len(hadiths), len(all_matches))
        if usable < len(hadiths):
            n_count_mismatch += (len(hadiths) - usable)
        if usable == 0:
            n_no_verdict += len(hadiths)
            continue

        for idx in range(usable):
            hadith_number = hadiths[idx]
            verdict_phrase = re.sub(r"\s+", " ", all_matches[idx][1])
            offset = all_matches[idx][0]
            bucket = normalize_grade(verdict_phrase)

            if not in_corpus(COLLECTION_ID, hadith_number):
                n_not_in_corpus += 1
                continue

            n_resolved += 1
            # Use a 200-char excerpt around the match for raw_text
            excerpt_start = max(0, offset - 20)
            rows.append({
                "hadith_number": hadith_number,
                "collection_id": COLLECTION_ID,
                "scholar_key": SCHOLAR_KEY,
                "scholar_ar": SCHOLAR_AR,
                "grade": verdict_phrase,
                "grade_normalized": bucket,
                "source_book_id": BOOK_ID,
                "source_page_index": page_index,
                "raw_text": block[excerpt_start:excerpt_start + 200],
                "notes": "extracted from Tuhfat al-Ahwadhi commentary",
            })

    print(f"  skipped (placeholder)  : {n_placeholder}")
    print(f"  attempted              : {n_attempted}")
    print(f"  resolved (rows)        : {n_resolved}")
    print(f"  no verdict on page     : {n_no_verdict}")
    print(f"  missing page in corpus : {n_missing_page}")
    print(f"  count mismatch (skip)  : {n_count_mismatch}")
    print(f"  not in our hadith set  : {n_not_in_corpus}")

    out = write_rows(BOOK_ID, rows)
    # write_rows uses a per-book filename; rename so it doesn't collide with
    # other potential book-21662 outputs (Tuhfat itself is reader-only here —
    # the rows are *Tirmidhi's verdicts*, sourced FROM Tuhfat).
    new_path = os.path.join(DATA_DIR, "grading_book_21662_tirmidhi_inline.json")
    if out != new_path:
        os.replace(out, new_path)
    print(f"\nWrote {len(rows)} rows to {new_path}")


if __name__ == "__main__":
    main()
