use rust_embed::RustEmbed;
use serde_json::json;

/// Compile-time embedded static distribution assets of the Svelte 5 admin dashboard.
#[derive(RustEmbed)]
#[folder = "ui/dist/"]
pub struct AdminUiAssets;

/// Single-Binary Embedded Admin Web Server for Aegis Gateway control plane.
pub struct EmbeddedAdminServer {
    pub port: u16,
    pub enabled: bool,
}

impl Default for EmbeddedAdminServer {
    fn default() -> Self {
        Self::new(8485, true)
    }
}

impl EmbeddedAdminServer {
    pub fn new(port: u16, enabled: bool) -> Self {
        Self { port, enabled }
    }

    /// Primary HTTP request dispatcher handling SPA asset serving and REST endpoints.
    pub fn handle_request(&self, method: &str, path: &str) -> (u16, Vec<u8>, &'static str) {
        if !self.enabled {
            return (
                404,
                b"Admin Web UI is disabled by policy".to_vec(),
                "text/plain",
            );
        }

        // 1. Dispatch REST API Endpoints
        if path.starts_with("/api/v1/") {
            return self.handle_api(method, path);
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
            // Client-side SPA fallback for paths like /dashboard, /dlp, /hitl, /finops
            (200, index.data.to_vec(), "text/html; charset=utf-8")
        } else {
            (404, b"Asset not found in embedded bundle".to_vec(), "text/plain")
        }
    }

    fn handle_api(&self, _method: &str, path: &str) -> (u16, Vec<u8>, &'static str) {
        match path {
            "/api/v1/overview" => {
                let payload = json!({
                    "status": "HEALTHY",
                    "rps": 1420,
                    "p99_latency_ms": 0.38,
                    "active_agents": 48,
                    "memory_rss_mb": 24.2,
                    "mtls_enforced": true
                });
                (200, payload.to_string().into_bytes(), "application/json")
            }
            "/api/v1/dlp/events" => {
                let payload = json!([
                    {
                        "id": "dlp_evt_1092",
                        "policy": "PCI-DSS v4.0",
                        "pattern": "Primary Account Number",
                        "action": "REDACTED_LUHN_MASK"
                    },
                    {
                        "id": "dlp_evt_1091",
                        "policy": "HIPAA PHI",
                        "pattern": "Medical Record Number",
                        "action": "HASH_CHAINED_ENCRYPT"
                    }
                ]);
                (200, payload.to_string().into_bytes(), "application/json")
            }
            "/api/v1/hitl/queue" => {
                let payload = json!([
                    {
                        "ticket_id": "hitl_tkt_8091",
                        "agent": "agent.finance.reconciler",
                        "action": "transfer_funds",
                        "risk_tier": "CRITICAL"
                    }
                ]);
                (200, payload.to_string().into_bytes(), "application/json")
            }
            "/api/v1/finops" => {
                let payload = json!({
                    "tenants_count": 3,
                    "frozen_tenants": ["tnt_scrape_08"],
                    "total_tokens_metered": "61.5M"
                });
                (200, payload.to_string().into_bytes(), "application/json")
            }
            _ => (404, json!({"error": "Endpoint not found"}).to_string().into_bytes(), "application/json"),
        }
    }

    /// Build an Axum Router serving static embedded SPA assets and REST APIs
    pub fn into_router(self: std::sync::Arc<Self>) -> axum::Router {
        axum::Router::new().fallback(move |req: axum::extract::Request| {
            let server = self.clone();
            async move {
                let method = req.method().as_str();
                let path = req.uri().path();
                let (status, body, mime) = server.handle_request(method, path);
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
