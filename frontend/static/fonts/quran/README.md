# QCF Per-Page Quran Fonts

Per-page glyph fonts used by [`AyahCard.svelte`](../../../src/lib/components/quran/AyahCard.svelte)
to render Quranic text in **Madani** and **Tajweed** modes (see
[`quranFonts.ts`](../../../src/lib/quranFonts.ts)).

## Layout

```
v2/   QPC V2 — black handwritten Uthmani glyphs (Madani mode)
v4/   QPC V4 — color-coded tajweed glyphs (Tajweed mode)
```

Each directory contains 604 files: `p1.woff2 … p604.woff2`, one per
mushaf page. Files are loaded on demand by `loadPageFont(page, mode)` —
only the pages the user actually views are fetched.

## Source

Fonts are sourced from [QUL (Tarteel.ai)](https://qul.tarteel.ai/resources/font),
which redistributes the originals from the **King Fahd Glorious Quran
Printing Complex** (Madinah, KSA).

| Variant | QUL resource | Origin |
|---|---|---|
| QPC V2 (Madani) | https://qul.tarteel.ai/resources/font/249 | KFGQPC |
| QPC V4 (Tajweed) | https://qul.tarteel.ai/resources/font/240 | KFGQPC |
| QPC Hafs (single, see `../UthmanicHafs.woff2`) | https://qul.tarteel.ai/resources/font/245 | KFGQPC |

## License

KFGQPC fonts are distributed for free for the purpose of displaying the
Quran. Redistribution as part of an application that displays Quranic
text — like this one — is the intended use. They are not under a SPDX
license; treat them as freely usable for Quran display, with attribution
to KFGQPC and QUL preserved in this README.

## Re-downloading

Fonts are committed to the repo so a fresh clone works out of the box.
If you ever need to refresh from upstream:

```sh
# Download both archives from QUL (browser, ZIP):
#   https://qul.tarteel.ai/resources/font/249  → QPC V2
#   https://qul.tarteel.ai/resources/font/240  → QPC V4

unzip <v2-zip> -d frontend/static/fonts/quran/v2
unzip <v4-zip> -d frontend/static/fonts/quran/v4
ls frontend/static/fonts/quran/v2 | wc -l   # expect 604
ls frontend/static/fonts/quran/v4 | wc -l   # expect 604
```

Filenames must be `p<N>.woff2` (1-indexed) to match `loadPageFont`.
