// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;

use crate::core::error::AegisResult;
use crate::core::policy::{PolicyContext, PolicyDecision, PolicyEngine};

pub mod approval_gate;
pub mod delegation;
pub mod egress;
pub mod infisical;
pub mod oauth_callback;
pub mod oauth_connect;
pub mod oauth_metadata;
pub mod oidc;
pub mod opa;
pub mod revocation;
pub mod secrets;
pub mod ssrf;

pub use approval_gate::ActionApprovalGate;
pub use delegation::{UserIdentityDelegationBroker, VirtualResourceScoper};
pub use egress::{DefaultEgressGuard, EgressPolicyGuard, EgressRuleConfig};
pub use infisical::{ChainedSecretStore, InfisicalSecretStore, InfisicalTransport, LocalInfisicalTransport};
pub use oauth_callback::{CallbackServerConfig, OAuthCallbackResolver};
pub use oauth_connect::VendorAgnosticOAuthRouter;
pub use oauth_metadata::ProtectedResourceMetadata;
pub use oidc::{OidcClaims, OidcTokenValidator};
pub use opa::{OpaRule, OpaPolicyEngine};
pub use revocation::MemoryRevocationRegistry;
pub use secrets::{EnvSecretStore, VaultSecretStore};
pub use ssrf::{SecurityPosture, SsrfRedirectValidator};

/// Enterprise Attribute-Based Access Control (ABAC) Policy Engine
#[derive(Default)]
pub struct AbacPolicyEngine;


impl AbacPolicyEngine {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PolicyEngine for AbacPolicyEngine {
    async fn evaluate(&self, ctx: &PolicyContext) -> AegisResult<PolicyDecision> {
        // 1. Role-based check
        let is_admin = ctx.caller.roles.iter().any(|r| r == "admin");
        let is_developer = ctx.caller.roles.iter().any(|r| r == "developer");

        // Sensitive destructive operations require admin
        if ctx.tool.contains("delete") || ctx.tool.contains("drop") {
            if !is_admin {
                return Ok(PolicyDecision::Deny {
                    reason: format!("Destructive tool '{}' requires 'admin' role", ctx.tool),
                });
            }
        }

        // Production database query tools require developer or admin
        if ctx.tool.contains("prod_db") && !is_developer && !is_admin {
            return Ok(PolicyDecision::Deny {
                reason: format!("Tool '{}' requires 'developer' or 'admin' role", ctx.tool),
            });
        }

        // 2. Payload-level argument constraint inspection
        self.eval_payload(&ctx.tool, &ctx.arguments)
    }

    fn eval_payload(&self, tool: &str, arguments: &Value) -> AegisResult<PolicyDecision> {
        // Example: Payload restriction on transfer/refund tools
        if tool.contains("refund") || tool.contains("transfer") {
            if let Some(amount) = arguments.get("amount").and_then(Value::as_f64) {
                if amount > 1000.0 {
                    return Ok(PolicyDecision::Deny {
                        reason: format!("Payload violation: Transaction amount ${amount} exceeds maximum threshold of $1000.00"),
                    });
                }
            }
        }

        Ok(PolicyDecision::Allow)
    }
}
