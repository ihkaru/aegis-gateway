// SPDX-License-Identifier: MIT

use serde_json::{json, Value};
use crate::core::error::{AegisError, AegisResult};
use crate::core::policy::PolicyEngine;
use crate::core::types::{CallerContext, DisclosureTier, ToolDefinition};
use crate::discovery::hybrid_search::HybridSearchEngine;
use crate::discovery::planner::{ExecutionPlan, ExecutionPlanner, PlanStep};

/// Autonomous Goal Deconstruction and Step-by-Step Multi-Tool Plan Generator
pub struct GoalPlanner;

impl GoalPlanner {
    /// Deconstructs a high-level user goal into an ordered multi-step execution DAG plan
    pub fn plan_goal(goal: &str, catalog: &[ToolDefinition]) -> ExecutionPlan {
        let clauses = Self::segment_goal(goal);
        let mut engine = HybridSearchEngine::new();
        engine.index_tools("default", catalog.to_vec());

        let mut steps = Vec::new();

        for (idx, clause) in clauses.iter().enumerate() {
            let step_id = format!("step_{}", idx + 1);
            let search_res = engine.search(clause, DisclosureTier::L0, 3);

            if let Some(best_tool) = search_res.tools.first() {
                let tool_def = catalog.iter().find(|t| t.name == best_tool.name);
                let mut args = json!({});

                // Automated dynamic data-piping synthesis across dependency steps
                if idx > 0 {
                    let prev_step_id = format!("step_{}", idx);
                    if let Some(tool) = tool_def {
                        for param in &tool.required_params {
                            if param.contains("customer") {
                                args[param] = json!(format!("{{{{{prev_step_id}.output.customer_id}}}}"));
                            } else if param.contains("charge") || param.contains("invoice") {
                                args[param] = json!(format!("{{{{{prev_step_id}.output.charge_id}}}}"));
                            } else if param.contains("path") || param.contains("file") || param.contains("url") {
                                args[param] = json!(format!("{{{{{prev_step_id}.output.path}}}}"));
                            } else if param.contains("message") || param.contains("body") {
                                args[param] = json!(format!("Auto-generated output payload from {prev_step_id}"));
                            }
                        }
                    }
                }

                let depends_on = if idx > 0 {
                    vec![format!("step_{}", idx)]
                } else {
                    Vec::new()
                };

                steps.push(PlanStep {
                    id: step_id,
                    tool: best_tool.name.clone(),
                    server: Some(best_tool.server.clone()),
                    arguments: args,
                    depends_on,
                    description: clause.clone(),
                });
            }
        }

        let plan_id = format!("plan_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0));

        ExecutionPlan {
            plan_id,
            title: goal.to_string(),
            steps,
        }
    }

    /// Splits a compound natural language goal into sequential capability milestones
    fn segment_goal(goal: &str) -> Vec<String> {
        let lower = goal.to_lowercase();
        // Multilingual transition delimiters: Indonesian & English
        let delimiters = [
            "\n",
            ";",
            " -> ",
            " kemudian ",
            " lalu ",
            " setelah itu ",
            " dan lalu ",
            " after that ",
            " and then ",
            " then ",
            ", ",
        ];

        let mut segments = vec![goal.to_string()];

        for delim in delimiters {
            let mut next_segments = Vec::new();
            for seg in segments {
                if seg.to_lowercase().contains(delim) {
                    for part in seg.split(delim) {
                        let trimmed = part.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-').trim();
                        if !trimmed.is_empty() {
                            next_segments.push(trimmed.to_string());
                        }
                    }
                } else {
                    next_segments.push(seg);
                }
            }
            segments = next_segments;
        }

        // If goal still has only 1 clause, try splitting by comma or conjunction if long
        if segments.len() <= 1 && goal.len() > 60 {
            let fallback_delims = [", ", " dan ", " and "];
            for delim in fallback_delims {
                if lower.contains(delim) {
                    segments = goal.split(delim)
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    break;
                }
            }
        }

        segments
    }

    /// Main entrypoint handling both goal-based formulation and explicit execution plan preflight
    pub async fn process_planning_request(
        params: &Value,
        catalog: &[ToolDefinition],
        planner: &ExecutionPlanner,
        policy: &dyn PolicyEngine,
        caller: &CallerContext,
    ) -> AegisResult<Value> {
        let goal_opt = params.get("goal")
            .or_else(|| params.get("objective"))
            .and_then(|g| g.as_str());

        let plan = if let Some(goal) = goal_opt {
            Self::plan_goal(goal, catalog)
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| AegisError::Internal(format!("Invalid plan schema or missing goal: {e}")))?
        };

        let validation = planner.validate_plan(&plan, catalog, policy, caller).await;

        Ok(json!({
            "valid": validation.valid,
            "plan_id": plan.plan_id,
            "title": plan.title,
            "step_count": validation.step_count,
            "overall_risk": validation.overall_risk,
            "human_approval_required": validation.human_approval_required,
            "plan": plan,
            "step_validations": validation.step_validations,
            "errors": validation.errors
        }))
    }
}
