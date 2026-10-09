// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use crate::core::error::AegisResult;
use crate::core::types::CallerContext;

/// Context provided to the ABAC policy evaluator
#[derive(Debug, Clone)]
pub struct PolicyContext {
    pub caller: CallerContext,
    pub server: String,
    pub tool: String,
    pub arguments: Value,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
}

/// Pluggable policy engine (In-tree rules, OPA / Rego, AWS Cedar)
#[async_trait]
pub trait PolicyEngine: Send + Sync {
    /// Evaluate whether caller can invoke the given tool with specific arguments
    async fn evaluate(&self, ctx: &PolicyContext) -> AegisResult<PolicyDecision>;

    /// Evaluate argument payload constraints (e.g. max dollar amount, forbidden params)
    fn eval_payload(&self, tool: &str, arguments: &Value) -> AegisResult<PolicyDecision>;
}
