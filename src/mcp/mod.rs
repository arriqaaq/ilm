//! Model Context Protocol server (HTTP, mounted into the existing axum router).
//!
//! Exposes the corpus as MCP tools so LLM clients (Claude Code, Cursor,
//! VS Code, Zed, MCP Inspector, …) can drive the same operations the HTTP
//! layer serves at `/v1/*`. Both layers call into `crate::services` — there
//! is no MCP-only logic.
//!
//! Wiring lives in [`http::build_mcp_router`]; `web::serve` mounts it via
//! `nest_service("/mcp", build_mcp_router(state))`.

pub mod http;
pub mod tools;

use rmcp::ErrorData as McpError;

use crate::web::AppState;

/// MCP server holding the shared `AppState`. Cheap to clone (all heavy state is `Arc`).
///
/// Tool methods, the constructor, and the `ServerHandler` impl all live in
/// [`crate::mcp::tools`] — `#[tool_router]` generates a module-private
/// function that all of those need to see.
#[derive(Clone)]
pub struct McpServer {
    pub state: AppState,
}

/// Convert internal `anyhow` errors to MCP protocol errors at the tool boundary.
pub(crate) fn mcp_err(e: anyhow::Error) -> McpError {
    McpError::internal_error(e.to_string(), None)
}
