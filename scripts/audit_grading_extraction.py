#!/usr/bin/env python3
"""
Audit script: for each extracted grading row, print the parser's claim
side-by-side with the actual book page text so the human can verify whether
the verdict was extracted correctly.

Output is written to stdout in a readable format. By default samples 10 rows
per (extraction file, source book). Pass --book BOOKID to focus, or
--samples N to change the count.

Usage:
  python3 scripts/audit_grading_extraction.py
  python3 scripts/audit_grading_extraction.py --book 1216
  python3 scripts/audit_grading_extraction.py --samples 5
"""

import argparse
import json
import os
import random
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_DIR = os.path.join(ROOT, "data")

# Map source_book_id → page-file slug, so we can pull actual page text.
BOOK_TO_SLUG = {
    1147:   "sahih_sunan_nasai",
    1148:   "daif_sunan_nasai",
    1216:   "daif_sunan_tirmidhi",
    25881:  "sahih_sunan_abi_dawud",
    5914:   "daif_sunan_abi_dawud",
    10757:  "sahih_jami_saghir",
    1663:   "daif_jami_saghir",
    9442:   "silsilat_sahihah",
    12762:  "silsilat_daifah",
    22592:  "irwa_al_ghalil",
    9082:   "ilal_daraqutni",
    123608: "talkhis_habir",
    1194:   "sunan_ibn_majah",
}

EXTRACTION_FILES = [
    "albani_sunan_grades.json",
    "abi_dawud_grades.json",
    "jami_cross_refs.json",
]


def strip_html(s):
    return re.sub(r"<[^>]+>", "", s)


def load_pages(slug):
    path = os.path.join(DATA_DIR, f"{slug}_pages.json")
    if not os.path.exists(path):
        return None
    with open(path, "r", encoding="utf-8") as f:
        pages = json.load(f)
    by_idx = {}
    for p in pages:
        pid = p.get("page_id")
        if isinstance(pid, int):
            by_idx[pid - 1] = strip_html(p.get("text", ""))
    return by_idx


def excerpt_around(text, needle, ctx=300):
    """Show ~ctx chars of text around the first occurrence of needle.
    If needle is None, return the first ctx chars.
    """
    if not text:
        return "(no page text available)"
    if needle:
        i = text.find(needle.strip()[:30]) if needle else -1
        if i >= 0:
            start = max(0, i - ctx // 2)
            end = min(len(text), i + len(needle) + ctx // 2)
            return ("..." if start > 0 else "") + text[start:end] + ("..." if end < len(text) else "")
    return text[: ctx * 2] + ("..." if len(text) > ctx * 2 else "")


ARABIC_DIGITS = "٠١٢٣٤٥٦٧٨٩"
WESTERN_DIGITS = "0123456789"
DIGIT_TRANS = str.maketrans(ARABIC_DIGITS, WESTERN_DIGITS)


def n_to_arabic(n):
    return str(n).translate(str.maketrans(WESTERN_DIGITS, ARABIC_DIGITS))


def audit_one_row(r, page_text):
    """Verify by finding the hadith_number anchor in the page text, then
    checking whether the parser's verdict appears between this anchor and
    the next entry header. Print VERDICT, NEXT-HADITH-DELTA, etc.
    """
    if not page_text:
        return "no_page"

    n = r["hadith_number"]
    n_ar = n_to_arabic(n)
    raw = (r.get("raw_text") or "").strip()

    # Find the entry's anchor: "<num> -" or "<baab> - <num> <chain word>".
    anchor_re = re.compile(rf"(?<!\d)(?:{re.escape(n_ar)}|{n})\s*(?:-|–)?\s*(?:عن|حدثنا|أخبرنا|قال|روى|إسناده)")
    am = anchor_re.search(page_text)
    if am is None:
        return f"NO ANCHOR for hadith#{n} on this page (parser claim suspicious)"

    # Find the NEXT hadith number anchor after this one.
    next_anchor_re = re.compile(r"(?<!\d)(\d+|[٠-٩]+)\s*(?:-|–)\s*(?:عن|حدثنا|أخبرنا|قال|روى)")
    after = page_text[am.end():]
    next_anchor = next_anchor_re.search(after)
    block = after[: next_anchor.start()] if next_anchor else after

    # Does the raw_text occur within `block`?
    short_raw = raw[:30] if raw else ""
    found_in_block = short_raw and (short_raw in block or short_raw[:20] in block)
    return (
        f"anchor@{am.start()}, block_len={len(block)}, "
        f"raw_in_block={'YES' if found_in_block else 'NO'} | "
        f"block[:200]={block[:200]!r}"
    )


def audit_file(path, samples, focus_book=None):
    if not os.path.exists(path):
        print(f"  {path}: not present, skipping")
        return
    with open(path, "r", encoding="utf-8") as f:
        rows = json.load(f)
    by_book = {}
    for r in rows:
        bid = r.get("source_book_id")
        if focus_book is not None and bid != focus_book:
            continue
        by_book.setdefault(bid, []).append(r)

    name = os.path.basename(path)
    print(f"\n{'='*72}\n  AUDIT: {name}  (total {len(rows)} rows)\n{'='*72}")

    for bid, brows in sorted(by_book.items(), key=lambda kv: (kv[0] is None, kv[0])):
        slug = BOOK_TO_SLUG.get(bid, "(unknown slug)") if bid else "(no source book)"
        pages = load_pages(slug) if bid else None
        sample = random.sample(brows, min(samples, len(brows)))
        nums = sorted(set(r["hadith_number"] for r in brows))
        print(f"\n--- book_id={bid}  slug={slug}  rows={len(brows)}  unique_hadith#={len(nums)}  ({len(sample)} sampled) ---")
        ok_count = 0
        bad_count = 0
        for r in sample:
            page_text = pages.get(r.get("source_page_index", -1)) if pages else None
            verdict = audit_one_row(r, page_text)
            ok = "raw_in_block=YES" in verdict
            ok_count += int(ok)
            bad_count += int(not ok and "raw_in_block=NO" in verdict)
            print(f"\n  Hadith #{r['hadith_number']:>5}  (page {r.get('source_page_index')})")
            print(f"    parser: grade='{r.get('grade','?')}' → {r.get('grade_normalized','?')}")
            print(f"    raw   : {(r.get('raw_text') or '')[:160]}")
            print(f"    audit : {'✓ MATCH' if ok else '✗ MISMATCH or no anchor'}  — {verdict[:300]}")
        print(f"\n  Summary for book {bid}: matches={ok_count}/{len(sample)}, mismatches={bad_count}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--samples", type=int, default=10)
    ap.add_argument("--book", type=int, default=None,
                    help="Only audit rows for this source_book_id")
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    random.seed(args.seed)
    for f in EXTRACTION_FILES:
        audit_file(os.path.join(DATA_DIR, f), args.samples, args.book)


if __name__ == "__main__":
    main()
