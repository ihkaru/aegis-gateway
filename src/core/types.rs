// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Progressive disclosure tiers for tool discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisclosureTier {
    /// L0: Name, one-line purpose (<= 120 chars), relevance score.
    L0,
    /// L1: L0 + functional signature, required arguments, and when-to-use guidance.
    L1,
    /// L2: Complete JSON input schema.
    L2,
}

/// Tenant identification for multi-tenancy
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub String);

impl TenantId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Caller context carrying authenticated identity, tenant, and role attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallerContext {
    pub tenant_id: TenantId,
    pub subject: String,
    pub roles: Vec<String>,
    pub department: Option<String>,
    pub client_ip: Option<String>,
    pub session_id: String,
}

/// Canonical MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub server: String,
    pub description: String,
    pub input_schema: Value,
    pub required_params: Vec<String>,
    pub when_to_use: String,
    pub tags: Vec<String>,
}

/// Projected tool definition adhering to progressive disclosure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectedTool {
    pub name: String,
    pub server: String,
    pub tier: DisclosureTier,
    pub summary: String,
    pub signature: Option<String>,
    pub required_params: Option<Vec<String>>,
    pub when_to_use: Option<String>,
    pub input_schema: Option<Value>,
    pub score: f64,
}

/// Tool execution call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub tool: String,
    pub server: String,
    pub arguments: Value,
    pub caller: CallerContext,
}

/// Tool execution call response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub success: bool,
    pub output: Value,
    pub latency_ms: u64,
    pub dlp_masked: bool,
}
