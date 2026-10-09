// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crate::core::error::AegisResult;
use crate::core::types::CallerContext;

/// Enterprise OIDC / SAML Identity Provider Token Validator
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validate bearer token, verify signatures, check expiry, and resolve CallerContext
    async fn validate_token(&self, token: &str) -> AegisResult<CallerContext>;
}
