// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::core::abac::{
    AbacContext, AbacDecision, AbacEffect, AbacPolicy, AbacPolicyEngine, AbacRule,
    PolicySimulationResult,
};
use crate::core::error::PropylonResult;

/// Pure-Rust In-Memory ABAC Policy Engine with Atomic Hot-Reload
#[derive(Debug, Clone)]
pub struct LocalAbacEngine {
    policy: Arc<RwLock<AbacPolicy>>,
}

impl LocalAbacEngine {
    pub fn new(default_policy: AbacPolicy) -> Self {
        Self {
            policy: Arc::new(RwLock::new(default_policy)),
        }
    }

    pub fn with_default_rules() -> Self {
        let rules = vec![
            AbacRule {
                name: "deny-destructive-mutations-by-default".to_string(),
                effect: AbacEffect::Deny,
                risk_tier: Some("critical".to_string()),
                allowed_departments: vec![],
                min_clearance: None,
                tool_patterns: vec!["drop_table".to_string(), "truncate_*".to_string()],
                allowed_backends: vec![],
                allowed_classifications: vec![],
                reason: Some("Destructive operations are forbidden for AI agents".to_string()),
            },
            AbacRule {
                name: "require-hitl-for-sensitive-mutations".to_string(),
                effect: AbacEffect::RequireHitl,
                risk_tier: Some("high".to_string()),
                allowed_departments: vec!["Engineering".to_string(), "DataOps".to_string()],
                min_clearance: Some("L2".to_string()),
                tool_patterns: vec!["mutate_*".to_string(), "execute_sql_mutation".to_string()],
                allowed_backends: vec![],
                allowed_classifications: vec!["internal".to_string()],
                reason: Some("Mutating actions require dual-custody human sign-off".to_string()),
            },
            AbacRule {
                name: "allow-engineering-read-analytics".to_string(),
                effect: AbacEffect::Allow,
                risk_tier: None,
                allowed_departments: vec!["Engineering".to_string(), "DataScience".to_string(), "Finance".to_string()],
                min_clearance: Some("L1".to_string()),
                tool_patterns: vec!["remote_analytics_*".to_string(), "read_*".to_string(), "query_*".to_string()],
                allowed_backends: vec![],
                allowed_classifications: vec!["public".to_string(), "internal".to_string()],
                reason: Some("Read-only queries allowed for verified departments".to_string()),
            },
        ];

        let policy = AbacPolicy {
            name: "default-enterprise-abac".to_string(),
            version: "v1.0.0".to_string(),
            default_effect: AbacEffect::Deny,
            rules,
        };

        Self::new(policy)
    }

    fn matches_clearance(user_level: &str, required: Option<&str>) -> bool {
        let Some(req) = required else {
            return true;
        };

        let rank = |lvl: &str| -> u8 {
            match lvl.to_uppercase().as_str() {
                "L1" => 1,
                "L2" => 2,
                "L3" => 3,
                "L4" => 4,
                "EXECUTIVE" => 5,
                _ => 0,
            }
        };

        rank(user_level) >= rank(req)
    }

    fn matches_wildcard(pattern: &str, target: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            return target.starts_with(prefix);
        }
        pattern == target
    }

    fn evaluate_rule(rule: &AbacRule, ctx: &AbacContext) -> bool {
        // 1. Department match
        if !rule.allowed_departments.is_empty()
            && !rule.allowed_departments.iter().any(|d| d.eq_ignore_ascii_case(&ctx.principal.department))
        {
            return false;
        }

        // 2. Clearance match
        if !Self::matches_clearance(&ctx.principal.clearance_level, rule.min_clearance.as_deref()) {
            return false;
        }

        // 3. Tool pattern match
        if !rule.tool_patterns.is_empty()
            && !rule.tool_patterns.iter().any(|p| Self::matches_wildcard(p, &ctx.action.tool_name))
        {
            return false;
        }

        // 4. Backend match
        if !rule.allowed_backends.is_empty()
            && !rule.allowed_backends.iter().any(|b| b.eq_ignore_ascii_case(&ctx.resource.backend_name))
        {
            return false;
        }

        // 5. Classification match
        if !rule.allowed_classifications.is_empty()
            && !rule.allowed_classifications.iter().any(|c| c.eq_ignore_ascii_case(&ctx.resource.data_classification))
        {
            return false;
        }

        true
    }
}

#[async_trait]
impl AbacPolicyEngine for LocalAbacEngine {
    async fn evaluate(&self, ctx: &AbacContext) -> PropylonResult<AbacDecision> {
        let policy_guard = self.policy.read().await;

        for rule in &policy_guard.rules {
            if Self::evaluate_rule(rule, ctx) {
                return Ok(AbacDecision {
                    effect: rule.effect,
                    matched_rule: Some(rule.name.clone()),
                    reason: rule.reason.clone().unwrap_or_else(|| format!("Matched rule '{}'", rule.name)),
                });
            }
        }

        // Default effect fallback
        Ok(AbacDecision {
            effect: policy_guard.default_effect,
            matched_rule: None,
            reason: format!("Default fallback effect: {:?}", policy_guard.default_effect),
        })
    }

    async fn simulate(&self, ctx: &AbacContext) -> PropylonResult<PolicySimulationResult> {
        let start = Instant::now();
        let policy_guard = self.policy.read().await;

        let mut matched_rules = Vec::new();
        let mut final_decision = None;

        for rule in &policy_guard.rules {
            if Self::evaluate_rule(rule, ctx) {
                matched_rules.push(rule.name.clone());
                if final_decision.is_none() {
                    final_decision = Some(AbacDecision {
                        effect: rule.effect,
                        matched_rule: Some(rule.name.clone()),
                        reason: rule.reason.clone().unwrap_or_else(|| format!("Matched rule '{}'", rule.name)),
                    });
                }
            }
        }

        let latency_micros = start.elapsed().as_micros() as u64;

        let decision = final_decision.unwrap_or_else(|| AbacDecision {
            effect: policy_guard.default_effect,
            matched_rule: None,
            reason: format!("Default fallback effect: {:?}", policy_guard.default_effect),
        });

        Ok(PolicySimulationResult {
            decision,
            matched_rules,
            evaluation_latency_micros: latency_micros,
        })
    }

    async fn reload_policy(&self, new_policy: AbacPolicy) -> PropylonResult<()> {
        let mut guard = self.policy.write().await;
        *guard = new_policy;
        Ok(())
    }

    async fn get_active_policy(&self) -> PropylonResult<AbacPolicy> {
        let guard = self.policy.read().await;
        Ok(guard.clone())
    }
}
