// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::{AegisError, AegisResult};
use crate::core::proxy::{CredentialProxyEngine, ProxyBinding};
use crate::core::secrets::SecretStore;

/// In-memory & loopback network credential proxy implementation
pub struct LoopbackCredentialProxy {
    secret_store: Arc<dyn SecretStore>,
    bindings: Arc<RwLock<HashMap<String, ProxyBinding>>>,
    base_port: u16,
}

impl LoopbackCredentialProxy {
    pub fn new(secret_store: Arc<dyn SecretStore>) -> Self {
        Self {
            secret_store,
            bindings: Arc::new(RwLock::new(HashMap::new())),
            base_port: 18080,
        }
    }

    pub fn with_base_port(mut self, port: u16) -> Self {
        self.base_port = port;
        self
    }
}

#[async_trait]
impl CredentialProxyEngine for LoopbackCredentialProxy {
    async fn bind_proxy(&self, session_id: &str) -> AegisResult<ProxyBinding> {
        let mut write = self.bindings.write().await;
        let port = self.base_port + (write.len() as u16 % 1000);
        let binding = ProxyBinding {
            proxy_url: format!("http://127.0.0.1:{port}"),
            session_id: session_id.to_string(),
            port,
        };
        write.insert(session_id.to_string(), binding.clone());
        Ok(binding)
    }

    async fn transform_request_headers(
        &self,
        service: &str,
        raw_headers: &[(String, String)],
    ) -> AegisResult<Vec<(String, String)>> {
        let mut headers: Vec<(String, String)> = raw_headers
            .iter()
            .filter(|(k, _)| !k.eq_ignore_ascii_case("authorization"))
            .cloned()
            .collect();

        let token_key = match service.to_lowercase().as_str() {
            "google" | "gdrive" | "gsheets" => "GOOGLE_ACCESS_TOKEN",
            "github" => "GITHUB_TOKEN",
            "aws" => "AWS_ACCESS_KEY_ID",
            _ => {
                return Ok(headers);
            }
        };

        if let Some(token) = self.secret_store.get_secret(token_key).await? {
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        } else {
            return Err(AegisError::SecretError(format!(
                "No credentials available for brokered service '{service}'"
            )));
        }

        Ok(headers)
    }

    async fn teardown_proxy(&self, session_id: &str) -> AegisResult<()> {
        let mut write = self.bindings.write().await;
        write.remove(session_id);
        Ok(())
    }
}
