// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;

use crate::core::error::{AegisError, AegisResult};
use crate::core::transport::{IngressTransport, WireProtocolHandler};

/// Streamable HTTP Transport processor (RFC 2025-03-26 single-endpoint architecture)
pub struct StreamableHttpTransport {
    handler: Arc<dyn WireProtocolHandler>,
    listen_addr: String,
}

impl StreamableHttpTransport {
    pub fn new(handler: Arc<dyn WireProtocolHandler>, listen_addr: impl Into<String>) -> Self {
        Self {
            handler,
            listen_addr: listen_addr.into(),
        }
    }

    pub fn listen_address(&self) -> &str {
        &self.listen_addr
    }

    /// Process single HTTP request payload (for integration in hyper/axum or embedded servers)
    pub async fn process_http_request(
        &self,
        path: &str,
        method: &str,
        body: &str,
    ) -> AegisResult<(u16, String, String)> {
        match (method, path) {
            // Modern RFC 2025-03-26 Streamable HTTP unified endpoint
            ("POST", "/mcp") => {
                if let Some(resp) = self.handler.handle_message(body).await? {
                    Ok((200, "application/json".to_string(), resp))
                } else {
                    Ok((204, "text/plain".to_string(), String::new()))
                }
            }
            // Streamable SSE request or legacy fallback endpoint
            ("GET", "/sse") | ("GET", "/mcp") => {
                let sse_init = "event: endpoint\ndata: /mcp\n\n";
                Ok((200, "text/event-stream".to_string(), sse_init.to_string()))
            }
            ("GET", "/health") | ("GET", "/healthz") => {
                Ok((200, "application/json".to_string(), r#"{"status":"healthy"}"#.to_string()))
            }
            _ => Err(AegisError::Internal(format!(
                "HTTP 404: Route not found for {method} {path}"
            ))),
        }
    }
}

#[async_trait]
impl IngressTransport for StreamableHttpTransport {
    async fn run(&self) -> AegisResult<()> {
        eprintln!("[AEGIS] Streamable HTTP transport listening on {}", self.listen_addr);
        Ok(())
    }
}
