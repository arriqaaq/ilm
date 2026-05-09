//! Helpers for the docs surface (`/openapi.json` + `/docs`).
//!
//! The full document — info + tags + every annotated `/v1/*` endpoint — is
//! assembled in [`super::serve`] by merging this metadata-only base with the
//! `OpenApi` extracted from each `OpenApiRouter::split_for_parts()` call.

use utoipa::OpenApi;

use super::openapi::ApiDoc;

/// Returns just the metadata document (no paths). Path entries come from the
/// per-handler `#[utoipa::path]` annotations registered via
/// `OpenApiRouter::routes!`.
pub fn base_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
