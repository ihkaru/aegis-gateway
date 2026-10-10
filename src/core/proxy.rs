// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crate::core::error::AegisResult;

/// Ephemeral proxy socket binding allocation
#[derive(Debug, Clone)]
pub struct ProxyBinding {
    pub proxy_url: String,
    pub session_id: String,
    pub port: u16,
}

/// Abstract contract for zero-knowledge loopback egress credential proxy
#[async_trait]
pub trait CredentialProxyEngine: Send + Sync {
    /// Bind an ephemeral loopback proxy for given execution session
    async fn bind_proxy(&self, session_id: &str) -> AegisResult<ProxyBinding>;

    /// Transform request headers to inject authorization token at transport layer
    async fn transform_request_headers(
        &self,
        service: &str,
        raw_headers: &[(String, String)],
    ) -> AegisResult<Vec<(String, String)>>;

    /// Teardown ephemeral proxy socket and flush internal buffers
    async fn teardown_proxy(&self, session_id: &str) -> AegisResult<()>;
}
