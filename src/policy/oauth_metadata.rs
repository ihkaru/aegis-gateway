//! OAuth 2.0 Protected Resource Metadata
//! Resolves microsoft/mcp-gateway#17 & microsoft/mcp-gateway#20 & docker/mcp-gateway#474
//! Implements RFC 8707 / OAuth 2.0 Protected Resource Metadata discovery endpoint.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtectedResourceMetadata {
    pub resource: String,
    pub authorization_servers: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub bearer_methods_supported: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_signing_alg_values_supported: Option<Vec<String>>,
}

impl ProtectedResourceMetadata {
    pub fn new(resource: impl Into<String>, auth_server: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            authorization_servers: vec![auth_server.into()],
            scopes_supported: vec![
                "mcp:read".to_string(),
                "mcp:write".to_string(),
                "mcp:admin".to_string(),
            ],
            bearer_methods_supported: vec!["header".to_string()],
            resource_documentation: None,
            resource_signing_alg_values_supported: Some(vec!["RS256".to_string(), "ES256".to_string()]),
        }
    }

    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes_supported = scopes;
        self
    }

    pub fn with_documentation(mut self, doc_url: impl Into<String>) -> Self {
        self.resource_documentation = Some(doc_url.into());
        self
    }

    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    pub fn validate_request_scope(&self, requested: &str) -> bool {
        self.scopes_supported.iter().any(|s| s == requested)
    }

    pub fn validate_audience(&self, aud: &str) -> bool {
        aud == self.resource || self.resource.starts_with(aud)
    }
}
