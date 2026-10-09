// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::backend::BackendTransport;
use crate::core::error::AegisResult;
use crate::core::transport::{JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, INTERNAL_ERROR};

/// Remote network-attached MCP backend adapter
pub struct RemoteHttpBackend {
    name: String,
    url: String,
    healthy: AtomicBool,
}

impl RemoteHttpBackend {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            healthy: AtomicBool::new(true),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

#[async_trait]
impl BackendTransport for RemoteHttpBackend {
    async fn start(&self) -> AegisResult<()> {
        self.healthy.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn send_request(&self, request: &JsonRpcRequest) -> AegisResult<JsonRpcResponse> {
        if !self.healthy.load(Ordering::SeqCst) {
            return Ok(JsonRpcResponse::error(
                request.id.clone().unwrap_or(JsonRpcId::Null),
                JsonRpcError::new(
                    INTERNAL_ERROR,
                    format!("Remote backend '{}' is currently unavailable", self.name),
                ),
            ));
        }

        // Return simulated remote response (or connect via HTTP client if integrated)
        Ok(JsonRpcResponse::success(
            request.id.clone().unwrap_or(JsonRpcId::Null),
            serde_json::json!({
                "server": self.name,
                "remote_url": self.url,
                "status": "connected"
            }),
        ))
    }

    async fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::SeqCst)
    }

    async fn stop(&self) -> AegisResult<()> {
        self.healthy.store(false, Ordering::SeqCst);
        Ok(())
    }
}
