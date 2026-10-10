use std::sync::Arc;
use std::time::Instant;
use rust_embed::RustEmbed;

use crate::core::backend::BackendRegistry;
use crate::AegisGateway;

/// Compile-time embedded static distribution assets of the Svelte 5 admin dashboard.
#[derive(RustEmbed)]
#[folder = "ui/dist/"]
pub struct AdminUiAssets;

/// Single-Binary Embedded Admin Web Server for Aegis Gateway control plane.
pub struct EmbeddedAdminServer {
    pub port: u16,
    pub enabled: bool,
    start_time: Instant,
    gateway: Option<Arc<AegisGateway>>,
    registry: Option<Arc<dyn BackendRegistry>>,
    recent_invocations: Arc<std::sync::RwLock<Vec<serde_json::Value>>>,
    recent_dlp_events: Arc<std::sync::RwLock<Vec<serde_json::Value>>>,
    current_tier: Arc<std::sync::RwLock<String>>,
}

impl Default for EmbeddedAdminServer {
    fn default() -> Self {
        Self::new(8485, true)
    }
}

impl EmbeddedAdminServer {
    pub fn new(port: u16, enabled: bool) -> Self {
        Self {
            port,
            enabled,
            start_time: Instant::now(),
            gateway: None,
            registry: None,
            recent_invocations: Arc::new(std::sync::RwLock::new(Vec::new())),
            recent_dlp_events: Arc::new(std::sync::RwLock::new(Vec::new())),
            current_tier: Arc::new(std::sync::RwLock::new("Hybrid".to_string())),
        }
    }

    pub fn with_gateway(mut self, gateway: Arc<AegisGateway>) -> Self {
        self.gateway = Some(gateway);
        self
    }

    pub fn with_registry(mut self, registry: Arc<dyn BackendRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    pub fn record_invocation(&self, call: serde_json::Value) {
        if let Ok(mut write) = self.recent_invocations.write() {
            write.push(call);
            if write.len() > 50 {
                write.remove(0);
            }
        }
    }

    pub fn record_dlp_event(&self, evt: serde_json::Value) {
        if let Ok(mut write) = self.recent_dlp_events.write() {
            write.push(evt);
            if write.len() > 50 {
                write.remove(0);
            }
        }
    }

    /// Primary HTTP request dispatcher handling SPA asset serving and REST endpoints.
    pub async fn handle_request(&self, method: &str, path: &str) -> (u16, Vec<u8>, &'static str) {
        self.handle_request_with_body(method, path, &[]).await
    }

    /// Primary HTTP request dispatcher with body support for REST endpoints.
    pub async fn handle_request_with_body(&self, method: &str, path: &str, body: &[u8]) -> (u16, Vec<u8>, &'static str) {
        if !self.enabled {
            return (
                404,
                b"Admin Web UI is disabled by policy".to_vec(),
                "text/plain",
            );
        }

        // 1. Dispatch REST API Endpoints via WebApiDispatcher (SRP Pattern)
        if path.starts_with("/api/v1/") {
            return crate::control::web_api::WebApiDispatcher::dispatch(
                method,
                path,
                body,
                self.start_time,
                self.gateway.as_ref(),
                self.registry.as_ref(),
                &self.recent_invocations,
                &self.recent_dlp_events,
                &self.current_tier,
            ).await;
        }

        // 2. Serve Static Frontend SPA Assets with SPA Fallback
        let clean_path = path.trim_start_matches('/');
        let target_file = if clean_path.is_empty() || clean_path == "index.html" {
            "index.html"
        } else {
            clean_path
        };

        if let Some(asset) = AdminUiAssets::get(target_file) {
            let mime = guess_mime(target_file);
            (200, asset.data.to_vec(), mime)
        } else if let Some(index) = AdminUiAssets::get("index.html") {
            (200, index.data.to_vec(), "text/html; charset=utf-8")
        } else {
            (404, b"Asset not found in embedded bundle".to_vec(), "text/plain")
        }
    }

    /// Build an Axum Router serving static embedded SPA assets and REST APIs
    pub fn into_router(self: std::sync::Arc<Self>) -> axum::Router {
        axum::Router::new().fallback(move |req: axum::extract::Request| {
            let server = self.clone();
            async move {
                let method = req.method().as_str().to_string();
                let path = req.uri().path().to_string();
                let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024)
                    .await
                    .unwrap_or_default();
                let (status, body, mime) = server
                    .handle_request_with_body(&method, &path, &body_bytes)
                    .await;
                axum::response::Response::builder()
                    .status(status)
                    .header("content-type", mime)
                    .body(axum::body::Body::from(body))
                    .unwrap_or_else(|_| {
                        axum::response::IntoResponse::into_response(
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    })
            }
        })
    }

    /// Run the embedded admin web server as an independent Axum HTTP service
    pub async fn run_server(self, host: &str) -> crate::core::error::AegisResult<()> {
        let addr_str = format!("{host}:{}", self.port);
        let addr: std::net::SocketAddr = addr_str
            .parse()
            .map_err(|e| crate::core::error::AegisError::Internal(format!("Invalid UI socket address '{addr_str}': {e}")))?;

        let server_arc = std::sync::Arc::new(self);
        let app = server_arc.into_router();
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        eprintln!("[AEGIS] Embedded Admin Web UI listening on http://{addr}");
        axum::serve(listener, app).await?;
        Ok(())
    }
}

fn guess_mime(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") || path.ends_with(".mjs") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}
