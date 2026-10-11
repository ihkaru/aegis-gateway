// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::error::AegisResult;

/// Principal attributes extracted from OAuth 2.1 / SAML / OIDC tokens
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalAttributes {
    pub user_id: String,
    pub department: String,
    pub clearance_level: String, // e.g. "L1", "L2", "L3", "L4", "Executive"
    pub roles: Vec<String>,
    pub groups: Vec<String>,
    pub extra: HashMap<String, String>,
}

impl Default for PrincipalAttributes {
    fn default() -> Self {
        Self {
            user_id: "anonymous".to_string(),
            department: "Engineering".to_string(),
            clearance_level: "L1".to_string(),
            roles: vec!["developer".to_string()],
            groups: vec![],
            extra: HashMap::new(),
        }
    }
}

/// Action attributes representing the requested tool operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionAttributes {
    pub tool_name: String,
    pub operation_type: String, // "read", "write", "execute", "mutate", "delete"
}

/// Resource attributes describing target upstream systems
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceAttributes {
    pub backend_name: String,
    pub data_classification: String, // "public", "internal", "pci_dss", "phi"
}

/// Environmental attributes at invocation time
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentAttributes {
    pub client_ip: String,
    pub timestamp_utc: String,
    pub require_mfa: bool,
}

impl Default for EnvironmentAttributes {
    fn default() -> Self {
        Self {
            client_ip: "127.0.0.1".to_string(),
            timestamp_utc: "2026-10-11T12:00:00Z".to_string(),
            require_mfa: false,
        }
    }
}

/// 4-Dimensional evaluation context for ABAC decisions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbacContext {
    pub principal: PrincipalAttributes,
    pub action: ActionAttributes,
    pub resource: ResourceAttributes,
    pub environment: EnvironmentAttributes,
}

/// Outcome effect of an ABAC evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbacEffect {
    Allow,
    Deny,
    RequireHitl,
}

/// Declarative rule matching 4-dimensional attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbacRule {
    pub name: String,
    pub effect: AbacEffect,
    pub risk_tier: Option<String>,
    pub allowed_departments: Vec<String>,
    pub min_clearance: Option<String>,
    pub tool_patterns: Vec<String>, // supports prefix or wildcard like "query_*"
    pub allowed_backends: Vec<String>,
    pub allowed_classifications: Vec<String>,
    pub reason: Option<String>,
}

/// Root declarative policy document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbacPolicy {
    pub name: String,
    pub version: String,
    pub default_effect: AbacEffect,
    pub rules: Vec<AbacRule>,
}

/// Decision output with explanation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbacDecision {
    pub effect: AbacEffect,
    pub matched_rule: Option<String>,
    pub reason: String,
}

/// Dry-run simulation response for SecOps audit and Embedded UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySimulationResult {
    pub decision: AbacDecision,
    pub matched_rules: Vec<String>,
    pub evaluation_latency_micros: u64,
}

/// Interface contract for declarative ABAC evaluation and zero-downtime hot-reload
#[async_trait]
pub trait AbacPolicyEngine: Send + Sync {
    /// Evaluates 4-dimensional context deterministically
    async fn evaluate(&self, ctx: &AbacContext) -> AegisResult<AbacDecision>;

    /// Simulates evaluation for SecOps dry-run testing
    async fn simulate(&self, ctx: &AbacContext) -> AegisResult<PolicySimulationResult>;

    /// Atomically updates active rules in memory without restart
    async fn reload_policy(&self, policy: AbacPolicy) -> AegisResult<()>;

    /// Fetches the currently active policy document
    async fn get_active_policy(&self) -> AegisResult<AbacPolicy>;
}
