// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::AegisResult;
use crate::core::sandbox::CredentialBroker;
use crate::core::secrets::SecretStore;

/// Vault and Infisical Credential Broker with short-lived memory injection and output redaction
pub struct VaultCredentialBroker {
    secret_store: Arc<dyn SecretStore>,
    redaction_dictionary: Arc<RwLock<Vec<String>>>,
}

impl VaultCredentialBroker {
    pub fn new(secret_store: Arc<dyn SecretStore>) -> Self {
        Self {
            secret_store,
            redaction_dictionary: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_secret(&self, secret: &str) {
        if secret.trim().len() >= 6 {
            let mut dict = self.redaction_dictionary.write().await;
            if !dict.contains(&secret.to_string()) {
                dict.push(secret.to_string());
            }
        }
    }

    async fn broker_google(&self, envs: &mut HashMap<String, String>) -> AegisResult<()> {
        let candidate_keys = [
            "AINA_GDRIVE_TOKEN_JSON",
            "GOOGLE_CREDENTIALS",
            "GOOGLE_APPLICATION_CREDENTIALS",
            "GOOGLE_ACCESS_TOKEN",
        ];

        for key in candidate_keys {
            if let Some(val) = self.secret_store.get_secret(key).await? {
                if val.starts_with('{') {
                    envs.insert("GOOGLE_APPLICATION_CREDENTIALS_JSON".to_string(), val.clone());
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
                        if let Some(token) = parsed.get("token").or_else(|| parsed.get("access_token")).and_then(|t| t.as_str()) {
                            envs.insert("GOOGLE_ACCESS_TOKEN".to_string(), token.to_string());
                            self.register_secret(token).await;
                        }
                        if let Some(refresh) = parsed.get("refresh_token").and_then(|r| r.as_str()) {
                            envs.insert("GOOGLE_REFRESH_TOKEN".to_string(), refresh.to_string());
                            self.register_secret(refresh).await;
                        }
                        if let Some(secret) = parsed.get("client_secret").and_then(|s| s.as_str()) {
                            envs.insert("GOOGLE_CLIENT_SECRET".to_string(), secret.to_string());
                            self.register_secret(secret).await;
                        }
                        if let Some(cid) = parsed.get("client_id").and_then(|c| c.as_str()) {
                            envs.insert("GOOGLE_CLIENT_ID".to_string(), cid.to_string());
                        }
                    }
                } else {
                    envs.insert("GOOGLE_ACCESS_TOKEN".to_string(), val.clone());
                    self.register_secret(&val).await;
                }
                break;
            }
        }
        Ok(())
    }

    async fn broker_github(&self, envs: &mut HashMap<String, String>) -> AegisResult<()> {
        if let Some(val) = self.secret_store.get_secret("GITHUB_TOKEN").await? {
            envs.insert("GITHUB_TOKEN".to_string(), val.clone());
            self.register_secret(&val).await;
        }
        Ok(())
    }

    async fn broker_aws(&self, envs: &mut HashMap<String, String>) -> AegisResult<()> {
        if let Some(val) = self.secret_store.get_secret("AWS_ACCESS_KEY_ID").await? {
            envs.insert("AWS_ACCESS_KEY_ID".to_string(), val);
        }
        if let Some(val) = self.secret_store.get_secret("AWS_SECRET_ACCESS_KEY").await? {
            envs.insert("AWS_SECRET_ACCESS_KEY".to_string(), val.clone());
            self.register_secret(&val).await;
        }
        Ok(())
    }

    async fn broker_generic(&self, service: &str, envs: &mut HashMap<String, String>) -> AegisResult<()> {
        let clean = service.to_uppercase().replace(['-', '.'], "_");
        let token_key = format!("{clean}_TOKEN");
        let api_key = format!("{clean}_API_KEY");

        if let Some(val) = self.secret_store.get_secret(&token_key).await? {
            envs.insert(token_key, val.clone());
            self.register_secret(&val).await;
        } else if let Some(val) = self.secret_store.get_secret(&api_key).await? {
            envs.insert(api_key, val.clone());
            self.register_secret(&val).await;
        }
        Ok(())
    }
}

#[async_trait]
impl CredentialBroker for VaultCredentialBroker {
    async fn broker_credentials(&self, services: &[String]) -> AegisResult<HashMap<String, String>> {
        let mut envs = HashMap::new();
        for svc in services {
            match svc.to_lowercase().as_str() {
                "google" | "gdrive" | "gdocs" | "gsheets" => {
                    self.broker_google(&mut envs).await?;
                }
                "github" => {
                    self.broker_github(&mut envs).await?;
                }
                "aws" => {
                    self.broker_aws(&mut envs).await?;
                }
                other => {
                    self.broker_generic(other, &mut envs).await?;
                }
            }
        }
        Ok(envs)
    }

    async fn redact_secrets(&self, text: &str) -> String {
        let dict = self.redaction_dictionary.read().await;
        let mut sanitized = text.to_string();
        for secret in dict.iter() {
            if !secret.is_empty() && sanitized.contains(secret) {
                sanitized = sanitized.replace(secret, "[REDACTED_CREDENTIAL]");
            }
        }
        sanitized
    }
}
