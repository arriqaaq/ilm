<p align="center">
  <img src="img/ilm.svg" alt="Ilm — Islamic Knowledge Platform" width="700">
</p>

<p align="center">
  <strong>Search the Quran & Sunnah. <em>Deeply.</em></strong><br>
  A semantic search platform for Islamic scholarship — Quran with tafsir, 34K+ hadiths with narrator chains, and interactive isnad graphs.
</p>

---

> **[API Reference](docs/API.md)** — Versioned `/v1/*` REST API + OpenAPI spec at `/openapi.json` &nbsp;|&nbsp; **[Methodology & Algorithms](docs/METHODOLOGY.md)** — Mustalah al-hadith isnad analysis &nbsp;|&nbsp; **[Data Sources](docs/DATA_SOURCES.md)** — Dataset documentation
>
> See also Barmaver's [*Dismantling Orientalist Narratives*](https://www.academia.edu/143038577/Dismantling_Orientalist_Narratives_A_Critique_of_Orientalists_Approach_to_Hadith_with_special_focus_on_Juynboll) (2025, free on Academia.edu).

## Architecture

<p align="center">
  <img src="img/architecture.svg" alt="Architecture overview" width="700">
</p>

Rust backend serving a SvelteKit SPA, with SurrealDB as a unified graph + vector + full-text database. Embeddings via FastEmbed, LLM via local Ollama.

## Features

- **Quran Reader** — 114 surahs with Tajweed Arabic, Sahih International translation, expandable Tafsir Ibn Kathir per ayah
- **Hadith Explorer** — 34K+ hadiths from 926 books across the 6 canonical collections
- **Narrator Networks** — 18K+ narrators with interactive graph visualization; click any narrator to read their full biographical entry in Tahdhib al-Tahdhib
- **Hybrid Search** — BM25 full-text + 1024-dim semantic vectors fused with Reciprocal Rank Fusion
- **Ask AI (GraphRAG)** — Natural language Q&A grounded in Quran and Hadith via local Ollama, with isnad-aware context and narrator chain citations
- **Early Manuscripts** — Per-ayah high-resolution manuscript images from Corpus Coranicum (Berlin-Brandenburg Academy), viewable with zoom
- **Isnad Analysis** — Hadith family clustering, mustalah-based chain grading (sahih/hasan/da'eef), transmission breadth (mutawatir/mashhur/aziz/gharib), corroboration detection (mutaba'at/shawahid), word-level matn diffing
- **Personal Study Notes** — Annotate any ayah or hadith, collect evidence by topic with @mentions that embed Quran verses and hadiths inline, tag-based organization, color-coded highlights, and full-text search across your notes
- **Public REST API** — Versioned `/v1/*` surface with OpenAPI spec; interactive docs at `/docs` (Scalar). Hadith, narrators, isnad chains, mustalah analysis, multi-scholar gradings, Quran ayah/word/root/similar/tafsir, and streaming GraphRAG. See [docs/API.md](docs/API.md).

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v20+)
- [Ollama](https://ollama.ai/) — `ollama pull command-r7b-arabic && ollama serve`

### Build & Run

```bash
git clone https://github.com/arriqaaq/ilm.git && cd ilm
make pipeline-full            # ingest all data (hadiths, Quran, books, PageIndex trees)
make dev                      # build & start HTTP server at localhost:3000
```

> **Note:** SurrealDB's HNSW vector index requires extra stack space. When running `cargo run` directly (outside of `make`), set `RUST_MIN_STACK=8388608`. The Makefile handles this automatically.

### MCP server (for Claude Desktop / Code / Cursor / any MCP client)

The same corpus is exposed as **Model Context Protocol** tools — 61 tools across search, hadith, narrator, family/mustalah, Quran, tafsir, books, scholars, notes, and Ask/RAG. Once registered with an MCP-aware LLM client, the model can call these tools directly during a chat instead of you mediating between it and the HTTP API.

#### How it works (the interaction model)

```
┌─────────────────┐  stdin (JSON-RPC)  ┌──────────────────┐
│  Claude / LLM   │ ─────────────────► │   ilm MCP server │
│  client         │                    │   (this binary)  │
│                 │ ◄───────────────── │                  │
└─────────────────┘  stdout (JSON-RPC) └──────┬───────────┘
                                              │
                                              ▼
                                     SurrealKv DB (db_data/)
                                     (hadith, Quran, narrators,
                                      books, families, gradings, notes)
```

1. The client launches `hadith mcp` as a child process and talks to it over stdio. No network, no port — the MCP server only lives as long as the client session.
2. On startup the client calls `tools/list` and gets back all 61 tools with their JSON Schemas. Claude/Cursor surfaces them in the UI (you'll see them under the server name).
3. Mid-conversation, when the model decides it needs corpus data, it calls a tool (e.g. `search_hadith`, `get_hadith`, `analyze_family_mustalah`, `ask_unified`). The server runs the SurrealKv query, returns JSON, the model reads it and continues the conversation.
4. You type natural language ("compare the matn of Bukhari hadith 1 and Muslim hadith 5") — the model picks the right tools, you don't call them by hand.

#### Quick try (no client needed)

```bash
make mcp-inspect      # opens the official MCP Inspector at http://localhost:5173
                      # browse all tools, fire calls, see JSON responses live
```

#### Claude Desktop

```bash
make mcp-claude-config       # prints the exact JSON block, with absolute paths
```

Paste the printed block under `mcpServers` in:

- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux:** `~/.config/Claude/claude_desktop_config.json`

Final config looks like:

```json
{
  "mcpServers": {
    "ilm": {
      "command": "/abs/path/to/ilm/target/debug/hadith",
      "args": ["mcp", "--db-path", "/abs/path/to/ilm/db_data"]
    }
  }
}
```

Restart Claude Desktop. In a new chat, click the tools icon — `ilm` should appear with all 61 tools listed. Try: *"Show me Bukhari hadith 1 with its full chain of narrators."*

To enable Ask/RAG and semantic search inside Claude (otherwise only the text-search tools work), add LLM/embed flags to `args`:

```json
"args": ["mcp", "--db-path", "/abs/path/to/ilm/db_data",
         "--llm-model", "llama3.2", "--embed-model", "e5-small"]
```

#### Claude Code (CLI)

Add the server with one command — Claude Code reads MCP servers from its own config:

```bash
claude mcp add ilm \
  -- /abs/path/to/ilm/target/debug/hadith mcp --db-path /abs/path/to/ilm/db_data
```

Then in any session: `/mcp` to list connected servers, or just ask a corpus question and Claude Code will discover and call the tools.

#### Cursor

Cursor uses `~/.cursor/mcp.json` (global) or `<project>/.cursor/mcp.json` (per-project). Same shape as the Claude Desktop config:

```json
{
  "mcpServers": {
    "ilm": {
      "command": "/abs/path/to/ilm/target/debug/hadith",
      "args": ["mcp", "--db-path", "/abs/path/to/ilm/db_data"]
    }
  }
}
```

Restart Cursor; the tools show up in the Composer's tool picker.

#### VS Code (MCP-capable extensions like Continue)

Same pattern — point the extension's MCP config at the binary above.

#### Building it directly without `make`

```bash
cargo build --features advanced       # build once
./target/debug/hadith mcp --db-path db_data  \
    --llm-model llama3.2 --embed-model e5-small  # optional flags
```

Or `cargo run --features advanced -- mcp --db-path db_data` — but stdio is the JSON-RPC channel, so don't pipe other commands' output into it.

#### Example interaction

Once `ilm` is registered with Claude Desktop, ask something like:

> *"For the hadith with ID `bukhari:1`, show me the full chain of narrators, the multi-scholar grading verdicts, and any Quran verses linked to it."*

Claude will (autonomously, in this order):

1. Call `get_hadith {"id": "bukhari:1"}` → gets the hadith + narrators + linked ayahs.
2. Call `get_chain_graph {"id": "bukhari:1"}` → gets the chain visualization data.
3. Call `get_hadith_gradings {"id": "bukhari:1"}` → gets the multi-scholar verdicts.
4. Synthesize a single answer citing each piece by `hadith_number`, narrator name, and scholar.

You see the answer; Claude shows the tool calls in a collapsed panel for transparency.

#### Tools reference

Run `make mcp-inspect` and click `tools/list` for the live, fully-typed catalog. Highlights:

| Domain | Sample tools |
|---|---|
| **Search** | `search_hadith`, `search_quran`, `search_unified`, `search_isnad`, `search_hadith_and_narrators` |
| **Hadith** | `get_hadith`, `list_hadiths`, `get_chain_graph`, `get_hadith_gradings`, `compare_matn` |
| **Narrator** | `get_narrator`, `narrator_teachers`, `narrator_students`, `get_narrator_graph`, `list_common_narrators` |
| **Family / Mustalah** | `list_families`, `get_family`, `analyze_family_mustalah`, `get_mustalah_stats` |
| **Quran** | `list_surahs`, `get_surah`, `get_ayah_words`, `get_ayah_hadiths`, `search_quran_root`, `get_similar_ayahs` |
| **Tafsir / Books** | `get_ayah_tafsir`, `get_surah_tafsir_pages`, `list_books`, `get_book_pages` |
| **Ask / RAG** | `ask_hadith`, `ask_quran`, `ask_unified` |
| **Notes** | `list_notes`, `create_note`, `update_note`, `list_notebooks` (full CRUD) |

> **Heads up:** SurrealKv is single-writer — don't run `make mcp` and `make dev` against the same `db_data` directory at the same time.

## Data Sources

| Dataset | Records | Content |
|---|---|---|
| [SemanticHadith KG V2](https://github.com/A-Kamran/SemanticHadith-V2) | 34K hadiths | Knowledge graph with narrator chains across 6 canonical collections |
| [Sunnah.com](https://huggingface.co/datasets/meeAtif/hadith_datasets) | 33K translations | Human English for 6 canonical collections |
| [QUL (Tarteel)](https://qul.tarteel.ai/) | 6,236 ayahs | QPC Hafs Arabic + Sahih International English |
| [Tafsir Ibn Kathir](https://qul.tarteel.ai/resources/tafsir/35) | 6,236 ayahs | Classical exegesis in English (HTML) |
| [AR-Sanad](https://github.com/somaia02/Narrator-Disambiguation) | 18K narrators | Ibn Hajar reliability classifications (Taqrib al-Tahdhib) |

All datasets are auto-downloaded on first run. See [DATA_SOURCES.md](docs/DATA_SOURCES.md) for details.

## Ingest Pipeline

<p align="center">
  <img src="img/ingest-pipeline.svg" alt="Ingest pipeline" width="700">
</p>

Parses the SemanticHadith KG, builds the narrator graph, generates embeddings, and merges human English translations from sunnah.com. Use `--translate` to fill gaps with Ollama.

## Search

<p align="center">
  <img src="img/search-flow.svg" alt="Search flow" width="700">
</p>

Three modes: **Hybrid** (default — BM25 + vector via Reciprocal Rank Fusion), **Text** (substring match), and **Semantic** (pure vector similarity). Works across both Arabic and English text.

## Ask (GraphRAG)

<p align="center">
  <img src="img/graphrag-flow.svg" alt="GraphRAG flow" width="700">
</p>

Ask questions in natural language. The system classifies the question, retrieves relevant Quran ayahs and hadiths via vector search, traverses the narrator graph to reconstruct each isnad (chain of narration), and passes this as context to a local LLM that streams a grounded answer with citations.

## Graph Model

<p align="center">
  <img src="img/graph-model.svg" alt="Database graph model" width="700">
</p>

SurrealDB stores narrators, hadiths, books, and ayahs as documents connected by `heard_from`, `narrates`, `belongs_to`, and `references_hadith` graph edges — enabling isnad reconstruction, Quran-Hadith cross-referencing, and network analysis.

## Early Manuscripts

<p align="center">
  <img src="img/manuscript-sample.jpg" alt="Early Quranic manuscript — Berlin, Wetzstein II 1913" width="700">
  <br><em>Berlin, Staatsbibliothek: Wetzstein II 1913 — Surah 2:238</em>
</p>

Per-ayah manuscript images from [Corpus Coranicum](https://corpuscoranicum.de/) (Berlin-Brandenburg Academy of Sciences). Click "Manuscripts" on any ayah to view high-resolution scans of early Quranic manuscripts — fetched live from the Corpus Coranicum API.

## Personal Study Notes

<p align="center">
  <img src="img/notes.svg" alt="Personal Study Notes" width="700">
</p>

Annotate any ayah or hadith with personal notes. Collect evidence by topic using @mentions that embed Quran verses and hadiths inline as rich cards. Organize with tags and color-coded highlights. Notes are stored in a separate `user_note` table — safely deletable without impacting ingested data.

- **@Mentions** — type `@2:255` to embed a Quran ayah, `@im_1` for a hadith, or search narrators by name
- **Topic Collections** — save ayahs and hadiths from anywhere into named study notes via the "Save" button
- **Tags & Search** — tag notes for organization, search across all notes by content or tag
- **Color Highlights** — 5 color options (yellow, green, blue, pink, purple) for visual categorization
- **Rich Embeds** — embedded references show the actual Arabic text and translation inline

## Training Pipeline

<p align="center">
  <img src="img/training-pipeline.svg" alt="Training pipeline" width="700">
</p>

Fine-tune a domain-specific LLM on hadith and Quran data, then deploy it through the existing Ollama-based ask loop with zero backend changes. The pipeline generates ~1,400 ChatML Q&A pairs matching the exact RAG prompt pattern from `rag.rs`, fine-tunes via LoRA (MLX locally or Unsloth on Colab), and exports to GGUF for Ollama. See [TRAINING.md](docs/TRAINING.md) for the full guide.

## Tech Stack

| Layer | Technology | Purpose |
|---|---|---|
| Backend | Rust, Axum | HTTP server, JSON API |
| Database | SurrealDB (SurrealKV) | Graph + HNSW vectors + BM25 full-text |
| Embeddings | FastEmbed (bge-m3) | 1024-dim semantic vectors |
| Frontend | SvelteKit 2, Svelte 5 | SPA served as static files |
| Graph Viz | Cytoscape.js | Narrator network visualization |
| LLM | Ollama (local) | Translation fallback + GraphRAG Q&A |

## Contributing

```bash
git clone https://github.com/arriqaaq/ilm.git && cd ilm
make build
cargo run -- ingest --limit 5 --translate   # quick test data
cd frontend && npm run dev                   # hot reload at :5173
```

See [METHODOLOGY.md](docs/METHODOLOGY.md) for the scholarly framework and [DATA_SOURCES.md](docs/DATA_SOURCES.md) for dataset documentation.
