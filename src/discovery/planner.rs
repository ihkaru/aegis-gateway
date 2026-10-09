// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::core::policy::{PolicyContext, PolicyDecision, PolicyEngine};
use crate::core::types::{CallerContext, ToolDefinition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlanRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub tool: String,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub arguments: serde_json::Value,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub title: String,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStepValidation {
    pub step_id: String,
    pub tool: String,
    pub allowed: bool,
    pub policy_reason: Option<String>,
    pub risk: PlanRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanValidation {
    pub valid: bool,
    pub plan_id: String,
    pub step_count: usize,
    pub overall_risk: PlanRisk,
    pub human_approval_required: bool,
    pub step_validations: Vec<PlanStepValidation>,
    pub errors: Vec<String>,
}

/// Enterprise Execution Planner with DAG cycle validation and policy preflight
#[derive(Clone, Default)]
pub struct ExecutionPlanner;

impl ExecutionPlanner {
    pub fn new() -> Self {
        Self
    }

    /// Validate a multi-step agent execution plan before execution
    pub async fn validate_plan(
        &self,
        plan: &ExecutionPlan,
        catalog: &[ToolDefinition],
        policy: &dyn PolicyEngine,
        caller: &CallerContext,
    ) -> PlanValidation {
        let mut errors = Vec::new();
        let mut step_validations = Vec::new();
        let mut overall_risk = PlanRisk::Low;
        let mut human_approval_required = false;

        // 1. Validate uniqueness of step IDs
        let mut step_ids = HashSet::new();
        for step in &plan.steps {
            if !step_ids.insert(step.id.clone()) {
                errors.push(format!("Duplicate step ID '{}' in execution plan", step.id));
            }
        }

        // 2. DAG Cycle Detection and Dependency Verification
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for step in &plan.steps {
            for dep in &step.depends_on {
                if !step_ids.contains(dep) {
                    errors.push(format!("Step '{}' depends on non-existent step '{}'", step.id, dep));
                } else {
                    adj.entry(&step.id).or_default().push(dep);
                }
            }
        }

        if Self::has_cycle(&adj) {
            errors.push("Circular dependency detected in execution plan steps".to_string());
        }

        // 3. Tool Existence & ABAC Policy Preflight
        for step in &plan.steps {
            let tool_def = catalog.iter().find(|t| t.name == step.tool);
            let server_name = step
                .server
                .clone()
                .or_else(|| tool_def.map(|t| t.server.clone()))
                .unwrap_or_else(|| "unknown".to_string());

            if tool_def.is_none() {
                errors.push(format!("Tool '{}' in step '{}' not found in tool catalog", step.tool, step.id));
            }

            let policy_ctx = PolicyContext {
                caller: caller.clone(),
                server: server_name,
                tool: step.tool.clone(),
                arguments: step.arguments.clone(),
                requested_at: chrono::Utc::now(),
            };

            let (allowed, policy_reason) = match policy.evaluate(&policy_ctx).await {
                Ok(PolicyDecision::Allow) => (true, None),
                Ok(PolicyDecision::Deny { reason }) => (false, Some(reason)),
                Err(e) => (false, Some(format!("Policy evaluation error: {e}"))),
            };

            let step_risk = Self::assess_step_risk(&step.tool, &step.arguments);
            if step_risk > overall_risk {
                overall_risk = step_risk;
            }
            if step_risk >= PlanRisk::High {
                human_approval_required = true;
            }

            step_validations.push(PlanStepValidation {
                step_id: step.id.clone(),
                tool: step.tool.clone(),
                allowed,
                policy_reason,
                risk: step_risk,
            });
        }

        let any_policy_denied = step_validations.iter().any(|s| !s.allowed);
        let valid = errors.is_empty() && !any_policy_denied;

        PlanValidation {
            valid,
            plan_id: plan.plan_id.clone(),
            step_count: plan.steps.len(),
            overall_risk,
            human_approval_required,
            step_validations,
            errors,
        }
    }

    fn has_cycle(adj: &HashMap<&str, Vec<&str>>) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for &node in adj.keys() {
            if Self::dfs_cycle(node, adj, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    fn dfs_cycle<'a>(
        node: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        visited: &mut HashSet<&'a str>,
        rec_stack: &mut HashSet<&'a str>,
    ) -> bool {
        if rec_stack.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }

        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = adj.get(node) {
            for &next in neighbors {
                if Self::dfs_cycle(next, adj, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    fn assess_step_risk(tool: &str, arguments: &serde_json::Value) -> PlanRisk {
        let args_str = arguments.to_string().to_lowercase();
        let tool_lower = tool.to_lowercase();

        if args_str.contains("drop ")
            || args_str.contains("truncate ")
            || args_str.contains("rm -rf")
            || tool_lower.contains("delete")
            || tool_lower.contains("drop")
        {
            PlanRisk::Critical
        } else if tool_lower.contains("transfer")
            || tool_lower.contains("write")
            || tool_lower.contains("update")
            || tool_lower.contains("publish")
        {
            PlanRisk::High
        } else if tool_lower.contains("query")
            || tool_lower.contains("search")
            || tool_lower.contains("read")
            || tool_lower.contains("get")
            || tool_lower.contains("list")
            || tool_lower.contains("fetch")
            || (tool_lower.contains("sql") && args_str.contains("select"))
        {
            PlanRisk::Low
        } else {
            PlanRisk::Medium
        }
    }
}
