// SPDX-License-Identifier: MIT

use std::net::SocketAddr;
use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde_json::json;

use crate::core::error::AegisResult;
use crate::core::transport::WireProtocolHandler;
use crate::state::DrainCoordinator;

#[derive(Clone)]
struct AppState {
    handler: Arc<dyn WireProtocolHandler>,
    drain: Arc<DrainCoordinator>,
}

/// Live Axum-based Streamable HTTP server for network-attached AI clients & Kubernetes
pub struct LiveHttpServer {
    state: AppState,
    listen_addr: SocketAddr,
}

impl LiveHttpServer {
    pub fn new(
        handler: Arc<dyn WireProtocolHandler>,
        drain: Arc<DrainCoordinator>,
        host: &str,
        port: u16,
    ) -> AegisResult<Self> {
        let addr_str = format!("{host}:{port}");
        let listen_addr: SocketAddr = addr_str
            .parse()
            .map_err(|e| crate::core::error::AegisError::Internal(format!("Invalid socket address '{addr_str}': {e}")))?;

        Ok(Self {
            state: AppState { handler, drain },
            listen_addr,
        })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/mcp", post(handle_post_mcp).get(handle_sse))
            .route("/sse", get(handle_sse))
            .route("/health", get(handle_health))
            .route("/healthz", get(handle_health))
            .route("/readyz", get(handle_ready))
            .route("/stats", get(handle_stats))
            .with_state(self.state.clone())
    }

    /// Run the server until shutdown signal is received
    pub async fn run(self) -> AegisResult<()> {
        let router = self.router();
        let listener = tokio::net::TcpListener::bind(&self.listen_addr).await?;
        eprintln!("[AEGIS] Listening on Streamable HTTP http://{}", self.listen_addr);

        let drain_clone = self.state.drain.clone();
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                eprintln!("[AEGIS] Received shutdown signal; draining active requests...");
                drain_clone.signal_shutdown();
            })
            .await?;

        Ok(())
    }
}

async fn handle_post_mcp(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    if let Some(ver) = headers.get("mcp-protocol-version").and_then(|v| v.to_str().ok()) {
        if !matches!(ver, "2024-11-05" | "2025-11-25" | "2024-10-07") {
            let err_body = json!({
                "jsonrpc": "2.0",
                "id": serde_json::Value::Null,
                "error": {
                    "code": -32022,
                    "message": format!("Unsupported MCP protocol version '{ver}'. Supported versions: 2024-11-05, 2025-11-25, 2024-10-07")
                }
            }).to_string();
            return (
                StatusCode::BAD_REQUEST,
                [("content-type", "application/json")],
                err_body,
            )
                .into_response();
        }
    }

    let _guard = match state.drain.acquire_slot() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Gateway is shutting down",
            )
                .into_response();
        }
    };

    match state.handler.handle_message(&body).await {
        Ok(Some(resp_json)) => (
            StatusCode::OK,
            [("content-type", "application/json")],
            resp_json,
        )
            .into_response(),
        Ok(None) => (StatusCode::NO_CONTENT, "").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal error: {e}"),
        )
            .into_response(),
    }
}

async fn handle_sse() -> Response {
    (
        StatusCode::OK,
        [
            ("content-type", "text/event-stream"),
            ("cache-control", "no-cache"),
        ],
        "event: endpoint\ndata: /mcp\n\n",
    )
        .into_response()
}

async fn handle_health() -> Response {
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        json!({ "status": "healthy" }).to_string(),
    )
        .into_response()
}

async fn handle_ready() -> Response {
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        json!({ "status": "ready" }).to_string(),
    )
        .into_response()
}

async fn handle_stats(State(state): State<AppState>) -> Response {
    let inflight = state.drain.inflight_count();
    let stats = json!({
        "status": "operational",
        "inflight_tasks": inflight,
        "is_draining": state.drain.is_draining()
    });
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        stats.to_string(),
    )
        .into_response()
}
