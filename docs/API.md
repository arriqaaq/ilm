# Ilm API

Public REST API for Quran, hadith, narrators, isnad chains, and multi-scholar gradings.

The interactive reference at **`/docs`** (rendered by Scalar from the
auto-generated OpenAPI spec at `/openapi.json`) is the authoritative source —
it lists every endpoint, every parameter, and every response shape. This page
is the one-pager.

## Base URL & versioning

```
http://localhost:3000/v1
```

The `/v1/*` prefix is stable. Breaking changes ship behind a new prefix
(`/v2`, `/v3`, ...). Internal endpoints used by the bundled SvelteKit
frontend live under `/internal/*` and are intentionally undocumented.

## Auth

Open access — no API keys, no auth headers. Rate-limited per IP:

| Surface              | Limit              |
| -------------------- | ------------------ |
| Read endpoints       | ~60 req/min/IP     |
| `/v1/ask/*` and `/v1/books/{id}/ask` | ~10 req/min/IP — these hit the LLM |

When you exceed the budget you get `429 Too Many Requests`.

## Response shapes

- **Pagination:** every list endpoint accepts `?page=N&limit=N` (1-indexed)
  and returns `{ data, page, limit, has_more, total? }`.
- **JSON casing:** snake_case throughout.
- **Errors:** documented endpoints return `{ code, message }` with an
  appropriate HTTP status. Path-not-found and rate-limit errors come from the
  framework as plain bodies — clients should treat any non-2xx as an error
  regardless.

## ID conventions

- **Hadith IDs** are slugs `{collection_code}:{number}`, e.g. `bukhari:1`,
  `muslim:42`, `tirmidhi:5`. Tahwil variants suffix with `:vN`
  (`bukhari:1:v2`). Codes come from `src/ingest/books.rs` and are stable.
- **Surah/ayah** addressing uses two path segments: `/v1/quran/ayahs/2/255/...`.
- **Narrator IDs**, **family IDs**, **phrase IDs** are opaque strings.
- **Book IDs** are integers (matching the upstream Turath/Shamela book IDs).

## Endpoint groups (tags in `/docs`)

| Tag | Surface |
| --- | --- |
| **Hadith** | `/v1/collections`, `/v1/hadiths`, `/v1/hadiths/{id}`, `/v1/hadiths/{id}/chain`, `/v1/hadiths/{id}/gradings`, `/v1/hadiths/sharh-pages`, `/v1/hadiths/diff`, `/v1/scholars` |
| **Narrators** | `/v1/narrators`, `/v1/narrators/autocomplete`, `/v1/narrators/common`, `/v1/narrators/{id}`, `/v1/narrators/{id}/graph`, `/v1/narrators/{id}/books`, `/v1/narrators/{id}/isnad-role` |
| **Isnad** | `POST /v1/isnad/search` |
| **Families** | `/v1/families`, `/v1/families/{id}`, `/v1/families/{id}/export` |
| **Mustalah** | `/v1/families/{id}/mustalah`, `/v1/mustalah/stats` |
| **Quran** | `/v1/quran/meta`, `/v1/quran/surahs`, `/v1/quran/surahs/{n}`, `/v1/quran/ayahs`, `/v1/quran/ayahs/{s}/{a}/{words,hadiths,similar,tafsir,tafsirs}`, `/v1/quran/surahs/{n}/{tafsir-pages,hadith-counts,similar-counts}`, `/v1/quran/phrases/{id}`, `/v1/quran/roots/{root}`, `/v1/quran/reciters` |
| **Books** | `/v1/books`, `/v1/books?category=hadith_grading`, `/v1/books/{id}`, `/v1/books/{id}/pages`, `/v1/books/config` |
| **Search** | `/v1/search/hadith`, `/v1/search/quran`, `/v1/search/all` |
| **Ask** | `POST /v1/ask/{hadith,quran,all,tafsir}`, `POST /v1/books/{id}/ask` — all SSE streams |
| **Meta** | `/v1/config`, `/v1/stats` |

## Examples

### Fetch the opening surah

```bash
curl 'http://localhost:3000/v1/quran/surahs/1' | jq '.surah.name_en, (.ayahs | length)'
# "Al-Fatihah"
# 7
```

### Hybrid search across Quran + Hadith

```bash
curl 'http://localhost:3000/v1/search/all?q=patience&type=hybrid&limit=5' \
  | jq '{quran_count, hadith_count}'
```

### A hadith with its isnad chain

```bash
curl 'http://localhost:3000/v1/hadiths/bukhari:1' | jq '.hadith.text_en'
curl 'http://localhost:3000/v1/hadiths/bukhari:1/chain' | jq '.nodes | length'
```

### Multi-scholar verdicts on a hadith

```bash
curl 'http://localhost:3000/v1/hadiths/abudawud:1/gradings' \
  | jq '.gradings[] | {scholar_key, grade_normalized, source_book_id}'
```

For Bukhari and Muslim hadiths the response always prepends a synthetic
`{ scholar_key: "bukhari", grade_normalized: "sahih", notes: "consensus sahih" }`
row. Other rows carry `source_book_id` — fetch the original Arabic page via
`GET /v1/books/{source_book_id}/pages?start={source_page_index}&size=1`.

### Streaming Q&A (Quran)

```bash
curl -N -X POST 'http://localhost:3000/v1/ask/quran' \
  -H 'content-type: application/json' \
  -d '{"question":"What does the Quran say about gratitude?"}'
```

The first SSE event carries `{quran_sources: [...]}`; subsequent events
stream answer tokens; a final `{done: true}` event closes the stream.

### Word-by-word morphology

```bash
curl 'http://localhost:3000/v1/quran/ayahs/2/255/words' | jq '.[0]'
```

### List all hadith-grading source books

```bash
curl 'http://localhost:3000/v1/books?category=hadith_grading' | jq '.[].name_en'
```

## Generating clients

Point an OpenAPI generator at `/openapi.json`:

```bash
# TypeScript (one of many options)
npx @openapitools/openapi-generator-cli generate \
  -i http://localhost:3000/openapi.json \
  -g typescript-fetch \
  -o ./ilm-client
```

## Stability

`/v1` is committed to backwards compatibility within its lifetime. New fields
may be added to existing response shapes; existing fields will not change
type or be renamed. Path additions are non-breaking. Breaking changes ship
as `/v2`.
