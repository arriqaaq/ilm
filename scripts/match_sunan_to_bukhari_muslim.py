#!/usr/bin/env python3
"""Match each Sunan hadith (Abu Dawud / Tirmidhi / Nasa'i / Ibn Majah) to its
parallel narration in Sahih al-Bukhari or Sahih Muslim by matn-only Jaccard
similarity.

These are recorded as parallel-narration links — NOT as grade attributions.
A high matn match means the same prophetic teaching is also recorded in
Bukhari/Muslim; it does not mean Bukhari or Muslim graded *this* Sunan
narration. The UI shows them like "similar hadith" entries.

Output: data/hadith_parallels_bukhari_muslim.json
  Rows shaped for ingest_hadith_parallels():
    { hadith_number, collection_id, parallel_collection_id,
      parallel_hadith_number, score, source }
"""

from __future__ import annotations

import html
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
SEMANTIC_PATH = REPO_ROOT / "data" / "semantic_hadith.json"
OUT_PATH = REPO_ROOT / "data" / "hadith_parallels_bukhari_muslim.json"

CODE_TO_COLL = {"SB": 1, "SM": 2, "SD": 3, "JT": 4, "SN": 5, "IM": 6}
SUNAN_COLLS = {3, 4, 5, 6}
BM_COLLS = {1, 2}

# Thresholds (calibrated from dry runs — see ScratchPlan in repo).
MIN_JACCARD = 0.55
MIN_MARGIN = 0.10
MIN_MATN_WORDS = 5
MIN_PREFILTER_OVERLAP = 4

DIACRITICS = re.compile(r"[ً-ٰٟـ]")
NON_ARABIC = re.compile(r"[^؀-ۿ\s]")
WHITESPACE = re.compile(r"\s+")
QUOTED = re.compile(r'"([^"]+)"')
QAALA_SPLIT = re.compile(r"قَالَ\s*[:؛]?\s*|قال\s*[:؛]?\s*")
# Common stopwords + chain-marker words. We strip these from word-set comparisons
# because they appear in nearly every hadith and dilute the signal.
STOPWORDS = {
    "في", "من", "إلى", "على", "عن", "أن", "إن", "ما", "لا", "لم",
    "هذا", "هذه", "ذلك", "كان", "كانت", "يكون", "بن", "ابن",
    "قال", "قالت", "قالوا", "حدثنا", "أخبرنا", "أنبأنا", "حدثني",
    "الله", "رسول", "النبي", "صلى", "عليه", "وسلم",
    "أبو", "أبي", "أبا", "ابن", "بنت",
    "هريرة", "عمر", "علي", "عائشة", "عمرو", "أنس",
}


def normalize(s: str) -> str:
    s = html.unescape(s)
    s = DIACRITICS.sub("", s)
    s = NON_ARABIC.sub(" ", s)
    s = WHITESPACE.sub(" ", s).strip()
    return s


def extract_matn(text_ar: str) -> str:
    """Strip the chain. Prefer the longest quoted block (prophetic speech in
    semantic_hadith.json is wrapped in `&quot;...&quot;` after html.unescape).
    Fallback: text after the last `قال:`. Final fallback: full text."""
    s = html.unescape(text_ar)
    quoted = QUOTED.findall(s)
    if quoted:
        return normalize(max(quoted, key=len))
    parts = QAALA_SPLIT.split(s)
    if len(parts) > 1:
        return normalize(parts[-1])
    return normalize(s)


def word_set(matn: str) -> frozenset[str]:
    return frozenset(w for w in matn.split() if w not in STOPWORDS and len(w) > 1)


def jaccard(a: frozenset[str], b: frozenset[str]) -> float:
    if not a or not b:
        return 0.0
    inter = len(a & b)
    if not inter:
        return 0.0
    return inter / len(a | b)


def main() -> None:
    print(f"Loading {SEMANTIC_PATH}...", flush=True)
    data = json.loads(SEMANTIC_PATH.read_text())
    hadiths = data["hadiths"]
    print(f"  {len(hadiths)} total hadiths", flush=True)

    # Build per-collection lists: (hadith_number, matn_words, public_id)
    by_coll: dict[int, list[tuple[int, frozenset[str], str]]] = defaultdict(list)
    for k, h in hadiths.items():
        code = k.split("-")[0]
        coll = CODE_TO_COLL.get(code)
        if not coll:
            continue
        matn = extract_matn(h["textAr"])
        words = word_set(matn)
        if len(words) < MIN_MATN_WORDS:
            continue
        by_coll[coll].append((int(h["refNo"]), words, k))

    for coll, lst in sorted(by_coll.items()):
        print(f"  collection {coll}: {len(lst):,} hadiths with usable matn", flush=True)

    # Inverted index over Bukhari + Muslim: word -> list of bm_index
    bm = by_coll[1] + by_coll[2]
    bm_coll = [1] * len(by_coll[1]) + [2] * len(by_coll[2])

    print(f"\nBuilding inverted index over {len(bm):,} B/M hadiths...", flush=True)
    inv: dict[str, list[int]] = defaultdict(list)
    for i, (_, words, _) in enumerate(bm):
        for w in words:
            inv[w].append(i)
    print(f"  index covers {len(inv):,} unique words", flush=True)

    out_rows: list[dict] = []
    n_sunan = sum(len(by_coll[c]) for c in SUNAN_COLLS)
    print(f"\nMatching {n_sunan:,} Sunan hadiths against B/M...", flush=True)

    progress_step = max(1, n_sunan // 50)
    processed = 0
    matched = 0
    per_coll_match: dict[int, int] = defaultdict(int)

    for coll in sorted(SUNAN_COLLS):
        for hadith_num, words, slug in by_coll[coll]:
            processed += 1
            if processed % progress_step == 0:
                pct = 100.0 * processed / n_sunan
                print(
                    f"  {processed:,}/{n_sunan:,} ({pct:.1f}%) matched={matched:,}",
                    flush=True,
                )

            # Pre-filter: candidates sharing ≥ MIN_PREFILTER_OVERLAP words.
            counter: dict[int, int] = defaultdict(int)
            for w in words:
                for idx in inv.get(w, ()):
                    counter[idx] += 1
            candidates = [i for i, c in counter.items() if c >= MIN_PREFILTER_OVERLAP]
            if not candidates:
                continue

            # Score
            best_score = 0.0
            second_score = 0.0
            best_idx = -1
            for i in candidates:
                _, bm_words, _ = bm[i]
                s = jaccard(words, bm_words)
                if s > best_score:
                    second_score = best_score
                    best_score = s
                    best_idx = i
                elif s > second_score:
                    second_score = s

            if best_idx < 0:
                continue
            if best_score < MIN_JACCARD:
                continue
            if best_score - second_score < MIN_MARGIN:
                continue

            bm_num, _, _ = bm[best_idx]
            target_coll = bm_coll[best_idx]

            out_rows.append({
                "hadith_number": hadith_num,
                "collection_id": coll,
                "parallel_collection_id": target_coll,
                "parallel_hadith_number": bm_num,
                "score": round(best_score, 4),
                "source": "matn_jaccard",
            })
            matched += 1
            per_coll_match[coll] += 1

    print(f"\nDone. Matched {matched:,} of {n_sunan:,} Sunan hadiths "
          f"({100.0 * matched / n_sunan:.1f}%).", flush=True)
    for coll in sorted(SUNAN_COLLS):
        n = len(by_coll[coll])
        m = per_coll_match[coll]
        if n:
            print(f"  collection {coll}: {m:,}/{n:,} ({100.0 * m / n:.1f}%)")

    # Strip diagnostic fields from output (keep one trace key for QA).
    OUT_PATH.write_text(json.dumps(out_rows, ensure_ascii=False, indent=1))
    print(f"\nWrote {OUT_PATH} ({len(out_rows):,} rows)", flush=True)


if __name__ == "__main__":
    sys.exit(main() or 0)
