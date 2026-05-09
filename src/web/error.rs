//! Centralized HTTP error mapping for the web layer.
//!
//! `WebError` wraps an `anyhow::Error` and implements `IntoResponse`. It
//! consults the per-service `is_not_found` markers to surface 404 vs 500, and
//! emits the canonical `ApiError { code, message }` envelope that the
//! OpenAPI spec already documents (see [`crate::models::ApiError`]).
//!
//! Handlers can adopt this gradually: change their signature from
//! `Result<impl IntoResponse, StatusCode>` to `Result<Json<T>, WebError>`
//! and `?`-propagate the service `anyhow::Result<T>`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};

use crate::models::ApiError;

/// Boundary error type for HTTP handlers. Wraps any `anyhow::Error` and
/// classifies it (not_found / bad_request / internal) on the way out.
pub struct WebError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl WebError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: message.into(),
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "unavailable",
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for WebError {
    fn from(e: anyhow::Error) -> Self {
        // Consult each service's "not found" marker. We check by string
        // because `anyhow::Error` doesn't expose the underlying type without
        // downcast and the service errors are constructed with `anyhow!`.
        let msg = e.to_string();
        if crate::services::hadith::is_not_found(&e)
            || crate::services::narrator::is_not_found(&e)
            || crate::services::family::is_not_found(&e)
            || crate::services::quran::is_not_found(&e)
            || crate::services::book::is_not_found(&e)
        {
            return Self::not_found(msg);
        }
        tracing::error!("handler error: {e:?}");
        Self::internal(msg)
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let status = self.status;
        let body = ApiError {
            code: self.code.to_string(),
            message: self.message,
        };
        (status, Json(body)).into_response()
    }
}
