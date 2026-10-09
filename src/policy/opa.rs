// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;

use crate::core::error::AegisResult;
use crate::core::policy::{PolicyContext, PolicyDecision, PolicyEngine};

#[derive(Debug, Clone)]
pub struct OpaRule {
    pub name: String,
    pub tool_pattern: String,
    pub allowed_roles: Vec<String>,
    pub required_department: Option<String>,
    pub max_amount: Option<f64>,
    pub disallowed_keywords: Vec<String>,
}

/// Pluggable Open Policy Agent (OPA / Rego) and Cedar-compatible engine
pub struct OpaPolicyEngine {
    rules: Vec<OpaRule>,
    opa_endpoint: Option<String>,
}

impl OpaPolicyEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            opa_endpoint: None,
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.opa_endpoint = Some(endpoint.into());
        self
    }

    pub fn add_rule(mut self, rule: OpaRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Preconfigure standard enterprise baseline security rules
    pub fn with_enterprise_defaults() -> Self {
        Self::new()
            .add_rule(OpaRule {
                name: "financial-transfer-guard".to_string(),
                tool_pattern: "stripe_refund|wire_transfer|payout".to_string(),
                allowed_roles: vec!["finance_admin".to_string(), "billing_manager".to_string()],
                required_department: Some("Finance".to_string()),
                max_amount: Some(5_000.0),
                disallowed_keywords: vec![],
            })
            .add_rule(OpaRule {
                name: "sql-ddl-protection".to_string(),
                tool_pattern: "query_database|sql_execute".to_string(),
                allowed_roles: vec!["admin".to_string(), "dba".to_string(), "developer".to_string()],
                required_department: None,
                max_amount: None,
                disallowed_keywords: vec![
                    "DROP".to_string(),
                    "DELETE".to_string(),
                    "TRUNCATE".to_string(),
                    "ALTER TABLE".to_string(),
                ],
            })
    }
}

impl Default for OpaPolicyEngine {
    fn default() -> Self {
        Self::with_enterprise_defaults()
    }
}

#[async_trait]
impl PolicyEngine for OpaPolicyEngine {
    async fn evaluate(&self, ctx: &PolicyContext) -> AegisResult<PolicyDecision> {
        for rule in &self.rules {
            if ctx.tool.contains(&rule.tool_pattern) || rule.tool_pattern.contains(&ctx.tool) {
                // Verify Role

                let has_role = ctx.caller.roles.iter().any(|r| rule.allowed_roles.contains(r));
                if !has_role {
                    return Ok(PolicyDecision::Deny {
                        reason: format!(
                            "OPA Policy '{}' denied: Caller roles {:?} do not contain required {:?}",
                            rule.name, ctx.caller.roles, rule.allowed_roles
                        ),
                    });
                }

                // Verify Department if required
                if let Some(req_dept) = &rule.required_department {
                    if ctx.caller.department.as_deref() != Some(req_dept.as_str()) {
                        return Ok(PolicyDecision::Deny {
                            reason: format!(
                                "OPA Policy '{}' denied: Required department '{}', caller is '{:?}'",
                                rule.name, req_dept, ctx.caller.department
                            ),
                        });
                    }
                }
            }
        }

        Ok(PolicyDecision::Allow)
    }

    fn eval_payload(&self, tool: &str, arguments: &Value) -> AegisResult<PolicyDecision> {
        for rule in &self.rules {
            if tool.contains(&rule.tool_pattern) || rule.tool_pattern.contains(tool) {
                // 1. Check Amount Constraint
                if let Some(max) = rule.max_amount {
                    if let Some(amt) = arguments.get("amount").and_then(|v| v.as_f64()) {
                        if amt > max {
                            return Ok(PolicyDecision::Deny {
                                reason: format!(
                                    "Payload amount ${:.2} exceeds policy limit ${:.2} for tool '{}'",
                                    amt, max, tool
                                ),
                            });
                        }
                    }
                }

                // 2. Check Disallowed SQL Keywords in Payload Strings
                if !rule.disallowed_keywords.is_empty() {
                    let serialized = arguments.to_string().to_uppercase();
                    for keyword in &rule.disallowed_keywords {
                        if serialized.contains(&keyword.to_uppercase()) {
                            return Ok(PolicyDecision::Deny {
                                reason: format!(
                                    "Payload contains forbidden keyword '{}' blocked by rule '{}'",
                                    keyword, rule.name
                                ),
                            });
                        }
                    }
                }
            }
        }

        Ok(PolicyDecision::Allow)
    }
}
