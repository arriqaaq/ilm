//! OpenAPI document for the public `/v1/*` API.
//!
//! The actual paths are wired up via `utoipa-axum`'s `OpenApiRouter` in
//! [`super::v1_router`] — this module just declares the document metadata and
//! the schema components that aren't reachable through `#[utoipa::path]`
//! annotations alone (e.g. the `PaginatedResponse` aliases).

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Ilm API",
        version = "1.0.0",
        description = "Public REST API for the Ilm Islamic knowledge platform.\n\
                       \n\
                       Search the Quran, hadith, narrators, isnad chains and \
                       multi-scholar gradings. All endpoints under `/v1/*` are \
                       open access and rate-limited per IP (60 req/min default; \
                       10 req/min for `/v1/ask/*` LLM endpoints).\n\
                       \n\
                       Internal endpoints under `/internal/*` (notes, notebooks, \
                       link-preview, translate, admin writes) are NOT documented \
                       here — they're used only by the bundled SvelteKit frontend.",
        license(name = "MIT"),
        contact(name = "Ilm", url = "https://github.com/arriqaaq/ilm"),
    ),
    servers(
        (url = "/", description = "Same-origin (this server)")
    ),
    tags(
        (name = "Hadith",     description = "Collections, hadiths, isnad chains, multi-scholar gradings, and matn diff."),
        (name = "Narrators",  description = "Biographical data, autocomplete, network graphs, isnad role analysis, and book references."),
        (name = "Isnad",      description = "Search hadiths by narrator chain (loose or strict)."),
        (name = "Families",   description = "Hadith family clustering — variants of the same hadith and their export bundles."),
        (name = "Mustalah",   description = "Mustalah al-hadith analysis (breadth, continuity, pivots) over hadith families."),
        (name = "Quran",      description = "Surahs, ayahs, word-level morphology, root concordance, similar-ayah graph, reciters, tafsir."),
        (name = "Books",      description = "Source-text books (turath/shamela): tafsir, sharh, biography, hadith grading source books."),
        (name = "Search",     description = "Search endpoints across hadith, Quran, and unified."),
        (name = "Ask",        description = "Streaming GraphRAG question answering. Responses are SSE token streams; rate-limited harder than read endpoints."),
        (name = "Meta",       description = "Server capabilities and global counts."),
    ),
    components(schemas(
        crate::models::ApiError,
    )),
)]
pub struct ApiDoc;
