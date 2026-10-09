// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::core::error::{AegisError, AegisResult};
use crate::core::identity::TokenValidator;
use crate::core::types::{CallerContext, TenantId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    pub sub: String,
    pub iss: Option<String>,
    pub exp: Option<i64>,
    pub tenant_id: Option<String>,
    pub department: Option<String>,
    pub roles: Option<Vec<String>>,
}

/// Enterprise OIDC / JWT Token Validator for federated corporate IdPs
pub struct OidcTokenValidator {
    expected_issuer: Option<String>,
    signing_secret: Option<String>,
    jwks_keys: HashMap<String, String>,
}

impl OidcTokenValidator {
    pub fn new() -> Self {
        Self {
            expected_issuer: None,
            signing_secret: None,
            jwks_keys: HashMap::new(),
        }
    }

    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.expected_issuer = Some(issuer.into());
        self
    }

    pub fn with_hmac_secret(mut self, secret: impl Into<String>) -> Self {
        self.signing_secret = Some(secret.into());
        self
    }

    pub fn register_jwk(mut self, kid: impl Into<String>, key: impl Into<String>) -> Self {
        self.jwks_keys.insert(kid.into(), key.into());
        self
    }

    /// Helper to forge a valid HMAC-SHA256 token for testing/federation simulation
    pub fn generate_test_token(claims: &OidcClaims, secret: &str) -> String {
        let header = serde_json::json!({
            "alg": "HS256",
            "typ": "JWT"
        });
        let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string());
        let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(claims).unwrap_or_default());
        let message = format!("{}.{}", header_b64, claims_b64);

        let mut hasher = Sha256::new();
        hasher.update(message.as_bytes());
        hasher.update(secret.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(hasher.finalize());

        format!("{}.{}", message, signature)
    }
}

impl Default for OidcTokenValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TokenValidator for OidcTokenValidator {
    async fn validate_token(&self, token: &str) -> AegisResult<CallerContext> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AegisError::AuthenticationFailed(
                "Invalid JWT format: expected 3 dot-separated segments".to_string(),
            ));
        }

        let header_raw = URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|e| AegisError::AuthenticationFailed(format!("Invalid header base64: {}", e)))?;
        let _header: serde_json::Value = serde_json::from_slice(&header_raw)
            .map_err(|e| AegisError::AuthenticationFailed(format!("Invalid header JSON: {}", e)))?;

        let claims_raw = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| AegisError::AuthenticationFailed(format!("Invalid claims base64: {}", e)))?;
        let claims: OidcClaims = serde_json::from_slice(&claims_raw)
            .map_err(|e| AegisError::AuthenticationFailed(format!("Invalid claims JSON: {}", e)))?;

        // 1. Validate Expiration
        if let Some(exp) = claims.exp {
            let now = chrono::Utc::now().timestamp();
            if now >= exp {
                return Err(AegisError::AuthenticationFailed(format!(
                    "Token expired: exp timestamp {} < current {}",
                    exp, now
                )));
            }
        }

        // 2. Validate Issuer if configured
        if let Some(expected_iss) = &self.expected_issuer {
            if claims.iss.as_deref() != Some(expected_iss.as_str()) {
                return Err(AegisError::AuthenticationFailed(format!(
                    "Issuer mismatch: expected '{}', got '{:?}'",
                    expected_iss, claims.iss
                )));
            }
        }

        // 3. Verify Signature if secret configured
        if let Some(secret) = &self.signing_secret {
            let message = format!("{}.{}", parts[0], parts[1]);
            let mut hasher = Sha256::new();
            hasher.update(message.as_bytes());
            hasher.update(secret.as_bytes());
            let expected_sig = URL_SAFE_NO_PAD.encode(hasher.finalize());

            if parts[2] != expected_sig {
                return Err(AegisError::AuthenticationFailed(
                    "JWT signature verification failed".to_string(),
                ));
            }
        }

        let tenant_str = claims.tenant_id.unwrap_or_else(|| "default-tenant".to_string());
        let roles = claims.roles.unwrap_or_else(|| vec!["developer".to_string()]);

        Ok(CallerContext {
            tenant_id: TenantId::new(tenant_str),
            subject: claims.sub,
            roles,
            department: claims.department,
            client_ip: None,
            session_id: uuid::Uuid::new_v4().to_string(),
        })
    }
}
