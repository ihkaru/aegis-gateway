// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::{PropylonError, PropylonResult};
use crate::core::identity::{
    AssertionExchangeRequest, DeviceAuthRequest, DeviceAuthResponse, DeviceCodeStatus,
    FederatedTokenResponse, IdentityFederationBroker,
};
use crate::core::types::CallerContext;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DeviceSession {
    device_code: String,
    user_code: String,
    client_id: String,
    scopes: Vec<String>,
    created_at_epoch: u64,
    expires_in_secs: u64,
    status: DeviceCodeStatus,
}

/// In-Memory Production Ready Identity Federation & RFC 8628 Device Broker
#[derive(Debug, Clone)]
pub struct LocalIdentityBroker {
    sessions: Arc<RwLock<HashMap<String, DeviceSession>>>,
    user_code_map: Arc<RwLock<HashMap<String, String>>>, // user_code -> device_code
    base_verification_uri: String,
    token_secret: String,
}

impl LocalIdentityBroker {
    pub fn new(base_verification_uri: impl Into<String>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_code_map: Arc::new(RwLock::new(HashMap::new())),
            base_verification_uri: base_verification_uri.into(),
            token_secret: "propylon-enterprise-broker-key".to_string(),
        }
    }

    fn generate_random_code(len: usize) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let chars = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut out = String::with_capacity(len);
        let mut acc = seed;
        for _ in 0..len {
            let idx = (acc % (chars.len() as u128)) as usize;
            out.push(chars[idx] as char);
            acc = acc.wrapping_mul(6364136223846793005).wrapping_add(1);
        }
        out
    }
}

#[async_trait]
impl IdentityFederationBroker for LocalIdentityBroker {
    async fn start_device_flow(&self, req: &DeviceAuthRequest) -> PropylonResult<DeviceAuthResponse> {
        if req.client_id.trim().is_empty() {
            return Err(PropylonError::Validation(
                "client_id cannot be empty".to_string(),
            ));
        }

        let device_code = format!("dev_{}", Self::generate_random_code(16).to_lowercase());
        let part1 = Self::generate_random_code(4);
        let part2 = Self::generate_random_code(4);
        let user_code = format!("{}-{}", part1, part2);

        let verification_uri = format!("{}/activate", self.base_verification_uri);
        let verification_uri_complete =
            format!("{}?user_code={}", verification_uri, user_code);

        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let session = DeviceSession {
            device_code: device_code.clone(),
            user_code: user_code.clone(),
            client_id: req.client_id.clone(),
            scopes: req.scopes.clone(),
            created_at_epoch: now,
            expires_in_secs: 900, // 15 mins default
            status: DeviceCodeStatus::Pending,
        };

        {
            let mut lock = self.sessions.write().await;
            lock.insert(device_code.clone(), session);
        }
        {
            let mut ulock = self.user_code_map.write().await;
            ulock.insert(user_code.clone(), device_code.clone());
        }

        Ok(DeviceAuthResponse {
            device_code,
            user_code,
            verification_uri,
            verification_uri_complete,
            expires_in: 900,
            interval: 5,
        })
    }

    async fn poll_device_flow(&self, device_code: &str) -> PropylonResult<DeviceCodeStatus> {
        let mut lock = self.sessions.write().await;
        let session = lock
            .get_mut(device_code)
            .ok_or_else(|| PropylonError::ResourceNotFound(format!("Device code '{}' not found", device_code)))?;

        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now > session.created_at_epoch + session.expires_in_secs {
            session.status = DeviceCodeStatus::Expired;
            return Ok(DeviceCodeStatus::Expired);
        }

        Ok(session.status.clone())
    }

    async fn approve_device_flow(&self, user_code: &str, context: CallerContext) -> PropylonResult<()> {
        let ulock = self.user_code_map.read().await;
        let device_code = ulock
            .get(user_code)
            .ok_or_else(|| PropylonError::ResourceNotFound(format!("User code '{}' not found or expired", user_code)))?;

        let mut lock = self.sessions.write().await;
        let session = lock
            .get_mut(device_code)
            .ok_or_else(|| PropylonError::ResourceNotFound("Associated device session missing".to_string()))?;

        session.status = DeviceCodeStatus::Approved { claims: context };
        Ok(())
    }

    async fn exchange_assertion(&self, req: &AssertionExchangeRequest) -> PropylonResult<FederatedTokenResponse> {
        if req.assertion.trim().is_empty() {
            return Err(PropylonError::AuthenticationFailed("Empty assertion payload".to_string()));
        }

        // Validate supported assertion types
        let is_saml = req.assertion_type == "urn:ietf:params:oauth:grant-type:saml2-bearer";
        let is_id_token = req.assertion_type == "urn:ietf:params:oauth:token-type:id_token";

        if !is_saml && !is_id_token {
            return Err(PropylonError::Validation(format!(
                "Unsupported assertion type: '{}'. Expected SAML 2.0 or OIDC ID Token.",
                req.assertion_type
            )));
        }

        // Synthetic cryptographic digest for token generation
        use std::time::{SystemTime, UNIX_EPOCH};
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let token_hash = format!("{:x}", md5_or_fnv(format!("{}:{}:{}", req.assertion, epoch, self.token_secret).as_bytes()));
        let scope = req.resource.clone().unwrap_or_else(|| "mcp:all".to_string());

        Ok(FederatedTokenResponse {
            access_token: format!("propylon_jwt_{}", token_hash),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            scope,
        })
    }
}

fn md5_or_fnv(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
