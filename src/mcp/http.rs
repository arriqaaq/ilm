//! HTTP (Streamable-HTTP) transport for the MCP server.
//!
//! Mounts the same `McpServer` (and its `tool_router`) inside the existing
//! axum router via `nest_service("/mcp", ...)`. Lives in the same process as
//! the HTTP server so it shares the single `Surreal<Db>` handle — no second
//! SurrealKv lock.

use std::sync::Arc;

use axum::Router;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use crate::mcp::McpServer;
use crate::web::AppState;

/// Build an axum sub-router that serves the MCP Streamable-HTTP transport.
/// Caller mounts via `Router::new().nest_service("/mcp", build_mcp_router(state))`.
pub fn build_mcp_router(state: AppState) -> Router {
    let session_manager = Arc::new(LocalSessionManager::default());

    let svc = StreamableHttpService::new(
        // service_factory: invoked per session — clone the cheap Arc-based AppState.
        move || Ok(McpServer::new(state.clone())),
        session_manager,
        StreamableHttpServerConfig::default(),
    );

    Router::new().fallback_service(svc)
}
