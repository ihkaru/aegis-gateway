// SPDX-License-Identifier: MIT

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::{Arc, OnceLock, RwLock};
use uuid::Uuid;

use crate::core::error::{PropylonError, PropylonResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseJwtClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
    pub roles: Vec<String>,
    pub department: Option<String>,
    pub tenant: Option<String>,
}

pub struct JwtTokenService {
    secret: String,
    issuer: String,
    audience: String,
    revoked_jtis: Arc<RwLock<HashSet<String>>>,
}

static SHARED_JWT_SERVICE: OnceLock<Arc<JwtTokenService>> = OnceLock::new();

pub fn get_shared_jwt_service() -> &'static Arc<JwtTokenService> {
    SHARED_JWT_SERVICE.get_or_init(|| {
        let secret = std::env::var("PROPYLON_JWT_SECRET")
            .unwrap_or_else(|_| "propylon_enterprise_ciso_secret_key_2026_bps_ridm".to_string());
        let issuer = std::env::var("PROPYLON_JWT_ISSUER")
            .unwrap_or_else(|_| "https://mcp.dvlpid.my.id".to_string());
        let audience = std::env::var("PROPYLON_JWT_AUDIENCE")
            .unwrap_or_else(|_| "propylon-gateway".to_string());
        Arc::new(JwtTokenService::new(secret, issuer, audience))
    })
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        let mut h = Sha256::new();
        h.update(key);
        let res = h.finalize();
        k[..32].copy_from_slice(&res);
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    let mut k_ipad = [0u8; 64];
    let mut k_opad = [0u8; 64];
    for i in 0..64 {
        k_ipad[i] = k[i] ^ 0x36;
        k_opad[i] = k[i] ^ 0x5c;
    }

    let mut inner = Sha256::new();
    inner.update(&k_ipad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(&k_opad);
    outer.update(&inner_hash);
    outer.finalize().into()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn extract_jwt(token: &str) -> &str {
    if token.starts_with("eyJ") {
        token
    } else if let Some(idx) = token.find(".eyJ") {
        &token[idx + 1..]
    } else if let Some(idx) = token.find("eyJ") {
        &token[idx..]
    } else {
        token
    }
}

impl JwtTokenService {
    pub fn new(
        secret: impl Into<String>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
    ) -> Self {
        Self {
            secret: secret.into(),
            issuer: issuer.into(),
            audience: audience.into(),
            revoked_jtis: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn generate_token(
        &self,
        sub: impl Into<String>,
        roles: Vec<String>,
        department: Option<String>,
        tenant: Option<String>,
        ttl_secs: i64,
    ) -> String {
        let now = chrono::Utc::now().timestamp();
        let jti = Uuid::new_v4().to_string();
        let claims = EnterpriseJwtClaims {
            sub: sub.into(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now,
            exp: now + ttl_secs,
            jti,
            roles,
            department,
            tenant,
        };

        let header = serde_json::json!({
            "alg": "HS256",
            "typ": "JWT"
        });
        let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string());
        let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&claims).unwrap_or_default());
        let message = format!("{}.{}", header_b64, claims_b64);

        let sig = hmac_sha256(self.secret.as_bytes(), message.as_bytes());
        let sig_b64 = URL_SAFE_NO_PAD.encode(sig);

        format!("{}.{}", message, sig_b64)
    }

    pub fn validate_token(&self, token: &str) -> PropylonResult<EnterpriseJwtClaims> {
        let clean_token = extract_jwt(token);
        let parts: Vec<&str> = clean_token.split('.').collect();
        if parts.len() != 3 {
            return Err(PropylonError::AuthenticationFailed(
                "Malformed JWT: must contain exactly 3 dot-separated segments".to_string(),
            ));
        }

        let message = format!("{}.{}", parts[0], parts[1]);
        let expected_sig = hmac_sha256(self.secret.as_bytes(), message.as_bytes());
        let actual_sig = URL_SAFE_NO_PAD
            .decode(parts[2])
            .map_err(|e| PropylonError::AuthenticationFailed(format!("Invalid signature encoding: {}", e)))?;

        if !constant_time_eq(&expected_sig, &actual_sig) {
            return Err(PropylonError::AuthenticationFailed(
                "Cryptographic signature mismatch: invalid HMAC-SHA256 signature".to_string(),
            ));
        }

        let claims_raw = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| PropylonError::AuthenticationFailed(format!("Invalid claims encoding: {}", e)))?;
        let claims: EnterpriseJwtClaims = serde_json::from_slice(&claims_raw)
            .map_err(|e| PropylonError::AuthenticationFailed(format!("Invalid claims JSON: {}", e)))?;

        let now = chrono::Utc::now().timestamp();
        if now >= claims.exp {
            return Err(PropylonError::AuthenticationFailed(format!(
                "Token expired: exp timestamp {} < current {}",
                claims.exp, now
            )));
        }

        if self.is_revoked(&claims.jti) || self.is_revoked(token) || self.is_revoked(clean_token) {
            return Err(PropylonError::AuthenticationFailed(format!(
                "Token revoked: jti '{}' has been revoked via enterprise kill-switch",
                claims.jti
            )));
        }

        if !self.issuer.is_empty() && claims.iss != self.issuer {
            return Err(PropylonError::AuthenticationFailed(format!(
                "Issuer mismatch: expected '{}', got '{}'",
                self.issuer, claims.iss
            )));
        }

        if !self.audience.is_empty() && claims.aud != self.audience {
            return Err(PropylonError::AuthenticationFailed(format!(
                "Audience mismatch: expected '{}', got '{}'",
                self.audience, claims.aud
            )));
        }

        Ok(claims)
    }

    pub fn refresh_token(&self, old_token: &str, ttl_secs: i64) -> PropylonResult<String> {
        let prefix = if old_token.starts_with("eyJ") {
            ""
        } else if let Some(idx) = old_token.find(".eyJ") {
            &old_token[..=idx]
        } else {
            ""
        };
        let clean = extract_jwt(old_token);
        let old_claims = self.validate_token(clean)?;
        self.revoke_jti(&old_claims.jti)?;
        let _ = self.revoke_jti(old_token);
        let _ = self.revoke_jti(clean);
        let raw_token = self.generate_token(
            old_claims.sub,
            old_claims.roles,
            old_claims.department,
            old_claims.tenant,
            ttl_secs,
        );
        if !prefix.is_empty() {
            Ok(format!("{}{}", prefix, raw_token))
        } else {
            Ok(raw_token)
        }
    }

    pub fn revoke_jti(&self, jti: &str) -> PropylonResult<()> {
        let mut lock = self.revoked_jtis.write().map_err(|_| {
            PropylonError::Internal("Failed to acquire write lock on revocation registry".to_string())
        })?;
        lock.insert(jti.to_string());
        Ok(())
    }

    pub fn revoke_token(&self, token: &str) -> PropylonResult<String> {
        let clean_token = extract_jwt(token);

        if let Ok(claims) = self.validate_token(clean_token) {
            self.revoke_jti(&claims.jti)?;
            let _ = self.revoke_jti(token);
            let _ = self.revoke_jti(clean_token);
            return Ok(claims.jti);
        }

        let parts: Vec<&str> = clean_token.split('.').collect();
        if parts.len() == 3 {
            if let Ok(claims_raw) = URL_SAFE_NO_PAD.decode(parts[1]) {
                if let Ok(claims) = serde_json::from_slice::<EnterpriseJwtClaims>(&claims_raw) {
                    self.revoke_jti(&claims.jti)?;
                    let _ = self.revoke_jti(token);
                    let _ = self.revoke_jti(clean_token);
                    return Ok(claims.jti);
                }
            }
        }

        self.revoke_jti(token)?;
        self.revoke_jti(clean_token)?;
        Ok(token.to_string())
    }

    pub fn is_revoked(&self, jti: &str) -> bool {
        self.revoked_jtis
            .read()
            .map(|set| set.contains(jti))
            .unwrap_or(false)
    }
}
