// SPDX-License-Identifier: MIT

use std::net::SocketAddr;
use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde_json::json;

use crate::core::error::PropylonResult;
use crate::core::transport::WireProtocolHandler;
use crate::state::DrainCoordinator;
use crate::transport::cors::{PropylonCorsPolicy, CorsPolicyEnforcer};

#[derive(Clone)]
struct AppState {
    handler: Arc<dyn WireProtocolHandler>,
    drain: Arc<DrainCoordinator>,
    cors: Arc<dyn CorsPolicyEnforcer>,
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
    ) -> PropylonResult<Self> {
        Self::with_cors(handler, drain, Arc::new(PropylonCorsPolicy::default()), host, port)
    }

    pub fn with_cors(
        handler: Arc<dyn WireProtocolHandler>,
        drain: Arc<DrainCoordinator>,
        cors: Arc<dyn CorsPolicyEnforcer>,
        host: &str,
        port: u16,
    ) -> PropylonResult<Self> {
        let addr_str = format!("{host}:{port}");
        let listen_addr: SocketAddr = addr_str
            .parse()
            .map_err(|e| crate::core::error::PropylonError::Internal(format!("Invalid socket address '{addr_str}': {e}")))?;

        Ok(Self {
            state: AppState { handler, drain, cors },
            listen_addr,
        })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route(
                "/mcp",
                post(handle_post_mcp)
                    .get(handle_sse)
                    .options(handle_options),
            )
            .route(
                "/sse",
                get(handle_sse)
                    .options(handle_options),
            )
            .route("/health", get(handle_health))
            .route("/healthz", get(handle_health))
            .route("/readyz", get(handle_ready))
            .route("/stats", get(handle_stats))
            .route(
                "/.well-known/oauth-protected-resource",
                get(handle_oauth_metadata),
            )
            .fallback(handle_fallback_ui_and_control_api)
            .with_state(self.state.clone())
    }

    /// Run the server until shutdown signal is received
    pub async fn run(self) -> PropylonResult<()> {
        let router = self.router();
        let listener = tokio::net::TcpListener::bind(&self.listen_addr).await?;
        eprintln!("[PROPYLON] Listening on Streamable HTTP http://{}", self.listen_addr);

        let drain_clone = self.state.drain.clone();
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                eprintln!("[PROPYLON] Received shutdown signal; draining active requests...");
                drain_clone.signal_shutdown();
            })
            .await?;

        Ok(())
    }
}

async fn handle_options(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let origin = headers.get("origin").and_then(|v| v.to_str().ok());
    let req_method = headers
        .get("access-control-request-method")
        .and_then(|v| v.to_str().ok());
    let req_headers = headers
        .get("access-control-request-headers")
        .and_then(|v| v.to_str().ok());

    let (status, resp_headers) = state.cors.evaluate_preflight(origin, req_method, req_headers);
    let mut resp = status.into_response();
    *resp.headers_mut() = resp_headers;
    resp
}

fn is_token_valid(auth_hdr: Option<&str>) -> bool {
    let Some(raw) = auth_hdr else { return false; };
    let clean = raw.strip_prefix("Bearer ").unwrap_or(raw).trim();
    if clean.is_empty() { return false; }
    let jwt_service = crate::policy::get_shared_jwt_service();
    if jwt_service.is_revoked(clean) {
        return false;
    }
    let candidate_jwt = if clean.starts_with("eyJ") {
        clean
    } else if let Some(idx) = clean.find(".eyJ") {
        &clean[idx + 1..]
    } else {
        ""
    };

    if !candidate_jwt.is_empty() {
        jwt_service.validate_token(candidate_jwt).is_ok()
    } else {
        clean.len() > 10
    }
}

async fn handle_post_mcp(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let origin = headers.get("origin").and_then(|v| v.to_str().ok());

    let tier_val = get_shared_policy_tier().read().map(|t| t.to_lowercase()).unwrap_or_default();
    let is_strict = tier_val == "strict"
        || std::env::var("PROPYLON_POLICY_TIER").map(|v| v.to_lowercase() == "strict").unwrap_or(false)
        || headers.get("x-propylon-policy-tier").and_then(|v| v.to_str().ok()) == Some("strict");
    if is_strict {
        let auth_hdr = headers.get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok());
        if !is_token_valid(auth_hdr) {
            let err_body = json!({
                "jsonrpc": "2.0",
                "id": serde_json::Value::Null,
                "error": {
                    "code": -32001,
                    "message": "Unauthorized: Strict Zero Trust requires SAML Bearer token. Login at https://mcp.dvlpid.my.id/saml/login or configure 'Authorization: Bearer <token>'."
                }
            }).to_string();
            let mut resp = (
                StatusCode::UNAUTHORIZED,
                [("content-type", "application/json")],
                err_body,
            ).into_response();
            state.cors.apply_cors_headers(resp.headers_mut(), origin);
            return resp;
        }
    }

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
            let mut resp = (
                StatusCode::BAD_REQUEST,
                [("content-type", "application/json")],
                err_body,
            )
                .into_response();
            state.cors.apply_cors_headers(resp.headers_mut(), origin);
            return resp;
        }
    }

    let _guard = match state.drain.acquire_slot() {
        Ok(g) => g,
        Err(_) => {
            let mut resp = (
                StatusCode::SERVICE_UNAVAILABLE,
                "Gateway is shutting down",
            )
                .into_response();
            state.cors.apply_cors_headers(resp.headers_mut(), origin);
            return resp;
        }
    };

    let mut response = match state.handler.handle_message(&body).await {
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
    };
    state.cors.apply_cors_headers(response.headers_mut(), origin);
    response
}

async fn handle_sse(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let origin = headers.get("origin").and_then(|v| v.to_str().ok());

    let tier_val = get_shared_policy_tier().read().map(|t| t.to_lowercase()).unwrap_or_default();
    let is_strict = tier_val == "strict"
        || std::env::var("PROPYLON_POLICY_TIER").map(|v| v.to_lowercase() == "strict").unwrap_or(false)
        || headers.get("x-propylon-policy-tier").and_then(|v| v.to_str().ok()) == Some("strict");
    if is_strict {
        let auth_hdr = headers.get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok());
        if !is_token_valid(auth_hdr) {
            let mut resp = (StatusCode::UNAUTHORIZED, "Unauthorized: Strict Zero Trust requires SAML Bearer token").into_response();
            state.cors.apply_cors_headers(resp.headers_mut(), origin);
            return resp;
        }
    }
    let mut resp = (
        StatusCode::OK,
        [
            ("content-type", "text/event-stream"),
            ("cache-control", "no-cache"),
        ],
        "event: endpoint\ndata: /mcp\n\n",
    )
        .into_response();
    state.cors.apply_cors_headers(resp.headers_mut(), origin);
    resp
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

async fn handle_oauth_metadata() -> Response {
    let metadata = crate::policy::ProtectedResourceMetadata::new(
        "http://localhost:39400/mcp",
        "https://auth.example.com",
    );
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        metadata.to_json().to_string(),
    )
        .into_response()
}

async fn handle_fallback_ui_and_control_api(
    uri: axum::http::Uri,
    method: axum::http::Method,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    crate::transport::fallback::handle_fallback_ui_and_control_api(uri, method, headers, body).await
}

pub fn get_shared_policy_tier() -> &'static std::sync::RwLock<String> {
    static TIER: std::sync::OnceLock<std::sync::RwLock<String>> = std::sync::OnceLock::new();
    TIER.get_or_init(|| {
        let initial = std::env::var("PROPYLON_POLICY_TIER")
            .unwrap_or_else(|_| "Hybrid".to_string());
        std::sync::RwLock::new(initial)
    })
}

pub fn get_shared_invocations() -> &'static std::sync::RwLock<Vec<serde_json::Value>> {
    static INVOCATIONS: std::sync::OnceLock<std::sync::RwLock<Vec<serde_json::Value>>> = std::sync::OnceLock::new();
    INVOCATIONS.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

pub fn get_shared_dlp_events() -> &'static std::sync::RwLock<Vec<serde_json::Value>> {
    static DLP: std::sync::OnceLock<std::sync::RwLock<Vec<serde_json::Value>>> = std::sync::OnceLock::new();
    DLP.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}
