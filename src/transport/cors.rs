// SPDX-License-Identifier: MIT

use axum::http::{HeaderMap, HeaderValue, StatusCode};

/// Outcome of CORS origin verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorsOriginOutcome {
    Allowed { origin: String, wildcard: bool },
    Disallowed(String),
    Omitted,
}

/// Interface-First Trait for Dynamic Browser CORS Enforcement (resolves docker/mcp-gateway#544)
pub trait CorsPolicyEnforcer: Send + Sync {
    /// Evaluate if an origin header is permitted
    fn evaluate_origin(&self, origin: Option<&str>) -> CorsOriginOutcome;

    /// Build response for HTTP OPTIONS preflight request
    fn evaluate_preflight(
        &self,
        origin: Option<&str>,
        requested_method: Option<&str>,
        requested_headers: Option<&str>,
    ) -> (StatusCode, HeaderMap);

    /// Apply standard CORS response headers to regular requests
    fn apply_cors_headers(&self, headers: &mut HeaderMap, origin: Option<&str>);
}

/// Standard Enterprise CORS Policy Implementation
#[derive(Debug, Clone)]
pub struct AegisCorsPolicy {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
    exposed_headers: Vec<String>,
    allow_credentials: bool,
    max_age_secs: u32,
}

impl Default for AegisCorsPolicy {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "OPTIONS".to_string(),
                "DELETE".to_string(),
            ],
            allowed_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "mcp-protocol-version".to_string(),
                "x-request-id".to_string(),
                "x-aegis-correlation-id".to_string(),
            ],
            exposed_headers: vec![
                "mcp-protocol-version".to_string(),
                "x-aegis-correlation-id".to_string(),
            ],
            allow_credentials: false,
            max_age_secs: 86400,
        }
    }
}

impl AegisCorsPolicy {
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_origins,
            ..Default::default()
        }
    }

    pub fn with_allow_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }

    pub fn with_allowed_headers(mut self, headers: Vec<String>) -> Self {
        self.allowed_headers = headers;
        self
    }

    pub fn is_origin_permitted(&self, origin: &str) -> bool {
        if self.allowed_origins.iter().any(|o| o == "*") {
            return true;
        }
        self.allowed_origins
            .iter()
            .any(|o| o.eq_ignore_ascii_case(origin.trim()))
    }
}

impl CorsPolicyEnforcer for AegisCorsPolicy {
    fn evaluate_origin(&self, origin: Option<&str>) -> CorsOriginOutcome {
        match origin {
            None => CorsOriginOutcome::Omitted,
            Some(org) => {
                let trimmed = org.trim();
                if self.is_origin_permitted(trimmed) {
                    let is_wildcard = self.allowed_origins.iter().any(|o| o == "*")
                        && !self.allow_credentials;
                    CorsOriginOutcome::Allowed {
                        origin: if is_wildcard {
                            "*".to_string()
                        } else {
                            trimmed.to_string()
                        },
                        wildcard: is_wildcard,
                    }
                } else {
                    CorsOriginOutcome::Disallowed(trimmed.to_string())
                }
            }
        }
    }

    fn evaluate_preflight(
        &self,
        origin: Option<&str>,
        _requested_method: Option<&str>,
        _requested_headers: Option<&str>,
    ) -> (StatusCode, HeaderMap) {
        let mut headers = HeaderMap::new();
        match self.evaluate_origin(origin) {
            CorsOriginOutcome::Allowed { origin: allowed_org, .. } => {
                if let Ok(val) = HeaderValue::from_str(&allowed_org) {
                    headers.insert("access-control-allow-origin", val);
                }

                let methods = self.allowed_methods.join(", ");
                if let Ok(val) = HeaderValue::from_str(&methods) {
                    headers.insert("access-control-allow-methods", val);
                }

                let req_headers = self.allowed_headers.join(", ");
                if let Ok(val) = HeaderValue::from_str(&req_headers) {
                    headers.insert("access-control-allow-headers", val);
                }

                let max_age = self.max_age_secs.to_string();
                if let Ok(val) = HeaderValue::from_str(&max_age) {
                    headers.insert("access-control-max-age", val);
                }

                if self.allow_credentials {
                    headers.insert(
                        "access-control-allow-credentials",
                        HeaderValue::from_static("true"),
                    );
                }

                (StatusCode::NO_CONTENT, headers)
            }
            CorsOriginOutcome::Disallowed(org) => {
                headers.insert(
                    "x-aegis-cors-rejection",
                    HeaderValue::from_str(&format!("Origin '{org}' not in whitelist"))
                        .unwrap_or(HeaderValue::from_static("rejected")),
                );
                (StatusCode::FORBIDDEN, headers)
            }
            CorsOriginOutcome::Omitted => (StatusCode::NO_CONTENT, headers),
        }
    }

    fn apply_cors_headers(&self, headers: &mut HeaderMap, origin: Option<&str>) {
        if let CorsOriginOutcome::Allowed { origin: allowed_org, .. } = self.evaluate_origin(origin) {
            if let Ok(val) = HeaderValue::from_str(&allowed_org) {
                headers.insert("access-control-allow-origin", val);
            }
            if !self.exposed_headers.is_empty() {
                let exp = self.exposed_headers.join(", ");
                if let Ok(val) = HeaderValue::from_str(&exp) {
                    headers.insert("access-control-expose-headers", val);
                }
            }
            if self.allow_credentials {
                headers.insert(
                    "access-control-allow-credentials",
                    HeaderValue::from_static("true"),
                );
            }
        }
    }
}
