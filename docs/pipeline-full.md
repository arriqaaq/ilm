# `make pipeline-full` — complete ingestion pipeline

End-to-end map of what `make pipeline-full` ingests, where every input comes
from, and what ends up in SurrealDB. Use this as the single source of truth
when adding a new ingestion step or debugging missing data.

The recipe lives at [Makefile:587-594](../Makefile#L587-L594) and runs five
stages, in order:

1. `quran-prepare` — build `data/quran.csv` from QUL JSON
2. `hadith-full` — ingest 6 hadith books + run analysis
3. `quran-full` — ingest Quran text + word morphology + similar-ayah graph
4. `book-full` — fetch + ingest 9 commentary / biography books, build PageIndex trees
5. `grading-full` — fetch + ingest 13 grading books, extract per-hadith verdicts

The two prerequisites (`data/semantic_hadith.json` and the `../PageIndex` repo)
are auto-built / auto-cloned. Run `make pipeline-check`
([Makefile:521-543](../Makefile#L521-L543)) first if you want a preflight
report.

---

## Stage 1 — `quran-prepare`

Builds `data/quran.csv` (columns: `surah,ayah,text_ar,text_en,tafsir_en`).

- Script: [scripts/prepare_quran_data.py](../scripts/prepare_quran_data.py)
- Inputs (committed in `qul/`, sourced from <https://qul.tarteel.ai/resources>):
  - `qul/qpc-hafs.json` — QPC Hafs Arabic (resources/quran-script/86)
  - `qul/taqi-ud-din-al-hilali-muhsin-khan-simple.json` — Hilali & Muhsin Khan English
  - `qul/en-tafisr-ibn-kathir.json` — Tafsir Ibn Kathir English HTML (resources/tafsir/35)
- Output: `data/quran.csv` (consumed by Stage 3a).

No DB writes in this stage.

---

## Stage 2 — `hadith-full`

Three substeps from [Makefile:93-111](../Makefile#L93-L111).

### 2a. `cargo run -- ingest`

- **Source**: `data/semantic_hadith.json`, derived from the SemanticHadith V2
  RDF/TTL knowledge graph at
  <https://github.com/A-Kamran/SemanticHadith-V2>. The file is auto-built by
  `make semantic-setup` (uses `uv run --with rdflib`).
- **Optional augmentation**: sunnah.com English translations (no Ollama
  needed; pulled directly during ingest).

Loads the canonical six (Kutub al-Sittah):

| `collection_id` | Name | Slug |
|---|---|---|
| 1 | Sahih al-Bukhari | bukhari |
| 2 | Sahih Muslim | muslim |
| 3 | Sunan Abi Dawud | abudawud |
| 4 | Jami at-Tirmidhi | tirmidhi |
| 5 | Sunan an-Nasa'i | nasai |
| 6 | Sunan Ibn Majah | ibnmajah |

Tables populated:

- [`narrator` — src/db.rs:38-56](../src/db.rs#L38-L56) — name_en/ar, kunya, aliases, generation, birth/death year + calendar, locations, hadith_count
- [`hadith` — src/db.rs:58-74](../src/db.rs#L58-L74) — hadith_number, collection_id, chapter_id, text_ar/en, narrator_text, grade, matn, topics, quran_verses, embedding (HNSW 1024d cosine)
- [`collection` — src/db.rs:76-80](../src/db.rs#L76-L80) — collection_id, name_en/ar
- [`narrates` — src/db.rs:133-136](../src/db.rs#L133-L136) — RELATION narrator → hadith (chain_position)
- [`heard_from` — src/db.rs:125-130](../src/db.rs#L125-L130) — RELATION narrator → narrator (hadith_ref, chain_position)
- [`belongs_to` — src/db.rs:139-140](../src/db.rs#L139-L140) — RELATION hadith → collection

### 2b. `analyze --families`

Module: [src/analysis/](../src/analysis/). Clusters hadith into variant
families using cosine similarity on `hadith.embedding` plus shared narrator
counts.

- [`hadith_family` — src/db.rs:84-89](../src/db.rs#L84-L89) — family_label, variant_count
- Updates `hadith.family_id` (record link)

### 2c. `analyze --mustalah`

Mustalah al-hadith (transmission criticism) analysis.

- [`isnad_analysis` — src/db.rs:94-101](../src/db.rs#L94-L101) — per family: breadth_class, min_breadth, bottleneck_tabaqah, chain_count, ilal_flags
- [`chain_assessment` — src/db.rs:103-109](../src/db.rs#L103-L109) — per variant: narrator_count, has_chronology_conflict
- [`narrator_pivot` — src/db.rs:111-120](../src/db.rs#L111-L120) — per (family, narrator): bundle_coverage, fan_out, collector_diversity, bypass_count, is_bottleneck

---

## Stage 3 — `quran-full`

Four substeps from [Makefile:241](../Makefile#L241):
`quran-check → quran (= prepare + ingest + hadith-refs) → morphology → similar`.

### 3a. `ingest-quran`

- Source: `data/quran.csv` (Stage 1).
- Tables:
  - [`surah` — src/db.rs:146-153](../src/db.rs#L146-L153) — surah_number, name_ar/en, name_translit, revelation_type, ayah_count
  - [`ayah` — src/db.rs:155-169](../src/db.rs#L155-L169) — surah_number, ayah_number, text_ar, text_ar_simple, text_ar_lemma, text_en, tafsir_en, juz, hizb, embedding (HNSW 1024d cosine). Composite UNIQUE on (surah_number, ayah_number).

### 3b. `ingest-quran-hadith-refs`

- **Source**: live Quran.com API —
  `https://quran.com/api/proxy/content/api/qdc/by-ayah?surah=<s>&ayah=<a>`.
  Fetched per-ayah at ingest time.
- Maps the API's collection slugs to our collection_id (bukhari→1, muslim→2,
  abudawud→3, tirmidhi→4, nasai→5, ibnmajah→6).
- Table: [`references_hadith` — src/db.rs:172-177](../src/db.rs#L172-L177) —
  RELATION ayah → hadith. Fields: collection, hadith_number, source='qurancom'.

### 3c. `ingest-morphology`

- **Sources**:
  - `data/quran-morphology.txt` — auto-fetched from
    <https://github.com/mustafa0x/quran-morphology> (a fork of the Quranic
    Arabic Corpus, corpus.quran.com).
  - Optional: `qul/colored-english-wbw-translation.json` for English glosses.
- Table: [`quran_word` — src/db.rs:183-200](../src/db.rs#L183-L200) —
  surah_number, ayah_number, word_position, text_ar, text_ar_simple,
  translation, transliteration, pos, root, lemma, features, segments.

### 3d. `ingest-quran-similar`

- **Sources** (committed `qul/` JSON, originally from qul.tarteel.ai):
  - `qul/phrases.json` — mutashabihat (resources/mutashabihat)
  - `qul/matching-ayah.json` — similar-ayah pairs (resources/similar-ayah)
- Tables:
  - [`quran_phrase` — src/db.rs:258-263](../src/db.rs#L258-L263) — text_ar, text_ar_simple, occurrence, verses_count, chapters_count
  - [`shares_phrase` — src/db.rs:266-271](../src/db.rs#L266-L271) — RELATION ayah → quran_phrase
  - [`similar_to` — src/db.rs:274-278](../src/db.rs#L274-L278) — RELATION ayah → ayah

> Manuscript imagery is served at runtime from the Corpus Coranicum live
> API, not ingested.

---

## Stage 4 — `book-full`

[Makefile:419](../Makefile#L419):
`turath-fetch → turath-mapping → turath-mapping-narrators → book-ingest → pageindex-build`.

### 4a. Fetch from turath.io

Each book is fetched by a dedicated Python script that calls the turath.io
API (`https://api.turath.io/book?id=<id>&include=indexes&ver=3`) using the
shared helper [scripts/_turath_fetch.py](../scripts/_turath_fetch.py).
Resume-safe; outputs `data/<slug>_pages.json` and `data/<slug>_headings.json`.

### 4b. Mapping scripts

Build the cross-reference JSONs that let the in-app reader jump from a
hadith / ayah / narrator to the right page in a commentary book. Inputs:
`data/semantic_hadith.json` + each book's headings JSON. Scripts:
`build_hadith_mapping.py` (Bukhari), `build_muslim_mapping.py`,
`build_tirmidhi_mapping.py`, `build_nasai_mapping.py`,
`build_abu_dawud_mapping.py`, `build_ibn_majah_mapping.py`,
`build_narrator_book_mapping.py` (Tahdhib).

### 4c. `ingest-turath` — books loaded

All 9 invocations live in [Makefile:299-413](../Makefile#L299-L413).

| Turath ID | Name (EN) | Name (AR) | Author | Category | Type | sharh_collection_id |
|---|---|---|---|---|---|---|
| 23604 | Tafsir Ibn Kathir | تفسير القرآن العظيم | ابن كثير | quran | tafsir | — |
| 7798  | Tafsir al-Tabari (Jami al-Bayan) | تفسير الطبري جامع البيان | الطبري | quran | tafsir | — |
| 1673  | Fath al-Bari | فتح الباري بشرح البخاري | ابن حجر العسقلاني | hadith | sharh | 1 (Bukhari) |
| 1711  | Sharh Nawawi on Muslim | شرح النووي على مسلم | النووي | hadith | sharh | 2 (Muslim) |
| 21662 | Tuhfat al-Ahwadhi | تحفة الأحوذي | المباركفوري | hadith | sharh | 4 (Tirmidhi) |
| 1147  | Sahih Sunan al-Nasa'i | صحيح سنن النسائي | الألباني | hadith | collection | 5 (Nasai) |
| 5760  | Awn al-Ma'bud | عون المعبود شرح سنن أبي داود | العظيم آبادي | hadith | sharh | 3 (Abu Dawud) |
| 98138 | Sunan Ibn Majah (Arnaut ed.) | سنن ابن ماجه - ت الأرنؤوط | ابن ماجه | hadith | collection | 6 (Ibn Majah) |
| 1278  | Tahdhib al-Tahdhib | تهذيب التهذيب | ابن حجر العسقلاني | narrator | biography | — |

Tables populated by every `ingest-turath` invocation:

- [`book` — src/db.rs:491-503](../src/db.rs#L491-L503) — book_id, name_ar/en, author_ar, total_pages, category, book_type, source='turath', source_id, tags
- [`book_page` — src/db.rs:505-511](../src/db.rs#L505-L511) — book_id, page_index, text, vol, page_num. UNIQUE on (book_id, page_index).

Plus, depending on `--*-mapping` flags:

- [`tafsir_ayah_map` — src/db.rs:513-520](../src/db.rs#L513-L520) — for tafsir books: surah, ayah, book_id, page_index, heading
- [`hadith_sharh_map` — src/db.rs:522-528](../src/db.rs#L522-L528) — for sharh / collection books: hadith_number, collection_id, book_id, page_index, context
- [`narrator_book_map` — src/db.rs:530-536](../src/db.rs#L530-L536) — for Tahdhib: narrator_id, book_id, page_index, entry_num, book_name

### 4d. `pageindex-build`

[scripts/index_books.py](../scripts/index_books.py) reads each book's
`*_pages.json` + `*_headings.json`, converts to markdown, and builds a
hierarchical retrieval tree using the sibling
[VectifyAI/PageIndex](https://github.com/VectifyAI/PageIndex) repo (auto-cloned
to `../PageIndex`). Outputs:

- `data/pageindex/<book_id>.json` — one tree per book
- `data/pageindex/book_map.json` — index metadata

No DB writes; trees are loaded at query time.

---

## Stage 5 — `grading-full`

Four substeps from [Makefile:443-465](../Makefile#L443-L465).

### 5a. `turath-fetch-grading`

[scripts/fetch_grading_books.py](../scripts/fetch_grading_books.py) — fetches
13 books from turath.io. The canonical list is at
[scripts/fetch_grading_books.py:30-44](../scripts/fetch_grading_books.py#L30-L44).

### 5b. `book-ingest-grading-books`

[scripts/ingest_grading_books.sh](../scripts/ingest_grading_books.sh) loads
each fetched book into `book` + `book_page` (same shape as Stage 4c) with
`book_type='grading'` (for grading books) or `'hadith_collection'` (for the
Sunan Ibn Majah source edition).

### 5c. `extract-grading`

One Python script per book parses page text into per-hadith verdict rows
written to `data/grading_book_<id>.json`. Some books are reader-only — pages
are loaded into `book_page` but no verdicts are extracted, because their
grades surface elsewhere (typically through Sahih/Daif Jami al-Saghir
cross-references).

### 5d. `book-ingest-grading-rows`

`cargo run -- ingest-grading` writes the extracted rows into:

- [`hadith_grading` — src/db.rs:572-589](../src/db.rs#L572-L589) — hadith_id (record<hadith>), scholar_key, scholar_ar, grade, grade_normalized, source_book_id, source_page_index, source_vol, source_page_num, raw_text, notes. Indexed by hadith_id and by source_book_id.

Idempotent: deletes prior rows for `source_book_id` before insert.

### Books ingested in `grading-full`

All 13 books, verbatim from
[scripts/fetch_grading_books.py:30-44](../scripts/fetch_grading_books.py#L30-L44):

| Turath ID | Name (EN) | Author | scholar_key | category | Verdict extractor |
|---|---|---|---|---|---|
| 1147   | Sahih Sunan al-Nasa'i | al-Albani | albani | hadith_grading | [extract_book_1147_sahih_nasai.py](../scripts/extract_book_1147_sahih_nasai.py) |
| 1148   | Daif Sunan al-Nasa'i | al-Albani | albani | hadith_grading | [extract_book_1148_daif_nasai.py](../scripts/extract_book_1148_daif_nasai.py) |
| 1216   | Daif Sunan al-Tirmidhi | al-Albani | albani | hadith_grading | [extract_book_1216_daif_tirmidhi.py](../scripts/extract_book_1216_daif_tirmidhi.py) |
| 25881  | Sahih Sunan Abi Dawud (Albani, Ghiras ed.) | al-Albani | albani | hadith_grading | [extract_book_25881_sahih_abi_dawud.py](../scripts/extract_book_25881_sahih_abi_dawud.py) |
| 5914   | Daif Sunan Abi Dawud al-Umm (Albani) | al-Albani | albani | hadith_grading | [extract_book_5914_daif_abi_dawud.py](../scripts/extract_book_5914_daif_abi_dawud.py) |
| 10757  | Sahih al-Jami al-Saghir (Albani) | al-Albani | albani | hadith_grading | [extract_book_10757_sahih_jami_saghir.py](../scripts/extract_book_10757_sahih_jami_saghir.py) — also recovers Sunan cross-refs |
| 1663   | Daif al-Jami al-Saghir (Albani) | al-Albani | albani | hadith_grading | [extract_book_1663_daif_jami_saghir.py](../scripts/extract_book_1663_daif_jami_saghir.py) (Suyuti numbering, reader-only verdicts) |
| 9442   | Silsilat al-Ahadith al-Sahihah | al-Albani | albani | hadith_grading | reader only |
| 12762  | Silsilat al-Ahadith al-Daifah | al-Albani | albani | hadith_grading | reader only |
| 22592  | Irwa al-Ghalil | al-Albani | albani | hadith_grading | reader only |
| 9082   | Ilal al-Daraqutni | al-Daraqutni | daraqutni | hadith_grading | reader only |
| 123608 | Talkhis al-Habir | Ibn Hajar al-Asqalani | ibn_hajar | hadith_grading | reader only |
| 1194   | Sunan Ibn Majah (Hadi ed., source) | Ibn Majah | source | hadith_collection | reader only |

Note (from the docstring of
[scripts/fetch_grading_books.py](../scripts/fetch_grading_books.py)): Sahih
Sunan al-Tirmidhi, Sahih/Daif Sunan Ibn Majah, and Sahih Sunan Abi Dawud
(Ma'arif ed.) are *not* present as standalone books on Turath — their
verdicts are recovered through Jami al-Saghir cross-references rather than
ingested directly.

---

## Complete book inventory

After `pipeline-full`, SurrealDB contains **6 hadith collections + 22
turath books** (9 commentary/biography + 13 grading), plus the Quran.

- **Hadith collections (6, in `collection`)**: Bukhari, Muslim, Abu Dawud, Tirmidhi, Nasa'i, Ibn Majah.
- **Commentary / biography books (9, in `book`)**: Tafsir Ibn Kathir, Tafsir al-Tabari, Fath al-Bari, Sharh Nawawi on Muslim, Tuhfat al-Ahwadhi, Sahih Sunan al-Nasa'i (Albani), Awn al-Ma'bud, Sunan Ibn Majah (Arnaut), Tahdhib al-Tahdhib.
- **Grading books (13, in `book` with `book_type='grading'` or 'hadith_collection')**: see Stage 5 table above.

Note: `1147` (Sahih Sunan al-Nasa'i, Albani) appears in both Stage 4 and
Stage 5 — it's ingested once (idempotent on `book_id`); Stage 4 loads it
with `--sharh-mapping` for in-reader hadith→page lookup, and Stage 5's
extractor produces grading rows pointing back at the same `book_page`s.

---

## Complete SurrealDB table inventory

All tables touched by `pipeline-full`, grouped by domain. Schemas live in
[src/db.rs](../src/db.rs) — line ranges below.

**Hadith core**
- [`narrator` (38-56)](../src/db.rs#L38-L56)
- [`hadith` (58-74)](../src/db.rs#L58-L74) — HNSW 1024d cosine on `embedding`
- [`collection` (76-80)](../src/db.rs#L76-L80)
- [`narrates` (133-136)](../src/db.rs#L133-L136) — RELATION
- [`heard_from` (125-130)](../src/db.rs#L125-L130) — RELATION
- [`belongs_to` (139-140)](../src/db.rs#L139-L140) — RELATION

**Hadith analysis**
- [`hadith_family` (84-89)](../src/db.rs#L84-L89)
- [`isnad_analysis` (94-101)](../src/db.rs#L94-L101)
- [`chain_assessment` (103-109)](../src/db.rs#L103-L109)
- [`narrator_pivot` (111-120)](../src/db.rs#L111-L120)

**Quran core**
- [`surah` (146-153)](../src/db.rs#L146-L153)
- [`ayah` (155-169)](../src/db.rs#L155-L169) — HNSW 1024d cosine on `embedding`; FULLTEXT on text_ar_simple, text_ar_lemma, text_en
- [`quran_word` (183-200)](../src/db.rs#L183-L200)
- [`quran_phrase` (258-263)](../src/db.rs#L258-L263)

**Quran edges**
- [`references_hadith` (172-177)](../src/db.rs#L172-L177) — ayah → hadith
- [`shares_phrase` (266-271)](../src/db.rs#L266-L271) — ayah → quran_phrase
- [`similar_to` (274-278)](../src/db.rs#L274-L278) — ayah → ayah

**Books / commentary**
- [`book` (491-503)](../src/db.rs#L491-L503)
- [`book_page` (505-511)](../src/db.rs#L505-L511)
- [`tafsir_ayah_map` (513-520)](../src/db.rs#L513-L520)
- [`hadith_sharh_map` (522-528)](../src/db.rs#L522-L528)
- [`narrator_book_map` (530-536)](../src/db.rs#L530-L536)

**Grading**
- [`hadith_grading` (572-589)](../src/db.rs#L572-L589)

HNSW vector indexes (`hadith_vec`, `ayah_vec`) and the FULLTEXT search
indexes are only created when the backend is built with the default
`advanced` feature; `--no-default-features` (lite mode) skips them.

---

## Source summary

| Domain | Source | Distribution |
|---|---|---|
| Quran text / English / English tafsir | <https://qul.tarteel.ai/resources> | committed in `qul/` |
| Quran word morphology | <https://github.com/mustafa0x/quran-morphology> | auto-fetched to `data/` |
| Mutashabihat / similar ayahs | qul.tarteel.ai (`phrases.json`, `matching-ayah.json`) | committed in `qul/` |
| Quran → hadith refs | quran.com API (live) | network call during ingest |
| Hadith corpus (6 books) | <https://github.com/A-Kamran/SemanticHadith-V2> (TTL) | auto-built to `data/semantic_hadith.json` |
| Hadith English translations (optional) | sunnah.com | network call during ingest |
| Commentary / biography books (9) | <https://api.turath.io> | fetched to `data/<slug>_{pages,headings}.json` |
| Grading books (13) | <https://api.turath.io> | fetched to `data/<slug>_{pages,headings}.json` |
| Quran manuscripts | corpuscoranicum.org (live, not ingested) | runtime API call only |
| PageIndex tree builder | <https://github.com/VectifyAI/PageIndex> (sibling clone) | `../PageIndex` |

All persisted data lives in the local SurrealDB store at `db_data/`. A clean
`make pipeline-full` from a fresh clone should populate it end to end with
no manual download steps for the data sources listed above (the QUL JSONs
and Quran fonts ship inside the repo).
