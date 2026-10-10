// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::future::Future;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::error::{AegisError, AegisResult};
use crate::discovery::planner::{ExecutionPlan, PlanRisk};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStepResult {
    pub step_id: String,
    pub tool: String,
    pub success: bool,
    pub output: Value,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineExecutionResult {
    pub plan_id: String,
    pub success: bool,
    pub total_steps: usize,
    pub completed_steps: usize,
    pub overall_risk: PlanRisk,
    pub step_results: Vec<PipelineStepResult>,
    pub step_outputs: HashMap<String, Value>,
    pub total_latency_ms: u64,
}

/// Dynamic argument expression resolver and DAG pipeline executor
#[derive(Clone, Default)]
pub struct DagPipelineEngine;

impl DagPipelineEngine {
    pub fn new() -> Self {
        Self
    }

    /// Recursively interpolates `{{step_id.path.to.key}}` expressions in argument JSON
    pub fn interpolate_value(val: &Value, outputs: &HashMap<String, Value>) -> AegisResult<Value> {
        match val {
            Value::String(s) => Self::interpolate_string(s, outputs),
            Value::Array(arr) => {
                let mut new_arr = Vec::with_capacity(arr.len());
                for item in arr {
                    new_arr.push(Self::interpolate_value(item, outputs)?);
                }
                Ok(Value::Array(new_arr))
            }
            Value::Object(map) => {
                let mut new_map = serde_json::Map::with_capacity(map.len());
                for (k, v) in map {
                    new_map.insert(k.clone(), Self::interpolate_value(v, outputs)?);
                }
                Ok(Value::Object(new_map))
            }
            _ => Ok(val.clone()),
        }
    }

    fn interpolate_string(s: &str, outputs: &HashMap<String, Value>) -> AegisResult<Value> {
        let trimmed = s.trim();
        // Exact single placeholder replacement: e.g. "{{step_1.output.id}}"
        if trimmed.starts_with("{{") && trimmed.ends_with("}}") && trimmed[2..trimmed.len() - 2].find("{{").is_none() {
            let expr = trimmed[2..trimmed.len() - 2].trim();
            return Self::resolve_expression(expr, outputs);
        }

        // Substring templating
        if !s.contains("{{") {
            return Ok(Value::String(s.to_string()));
        }

        let mut result = s.to_string();
        while let Some(start) = result.find("{{") {
            let end = match result[start..].find("}}") {
                Some(e) => start + e,
                None => break,
            };
            let expr = result[start + 2..end].trim();
            let resolved = Self::resolve_expression(expr, outputs)?;
            let replacement = match resolved {
                Value::String(ref str_val) => str_val.clone(),
                _ => resolved.to_string(),
            };
            result.replace_range(start..end + 2, &replacement);
        }

        Ok(Value::String(result))
    }

    fn resolve_expression(expr: &str, outputs: &HashMap<String, Value>) -> AegisResult<Value> {
        let parts: Vec<&str> = expr.split('.').collect();
        if parts.is_empty() {
            return Err(AegisError::Internal("Empty expression in template".to_string()));
        }

        let step_id = parts[0];
        let root = outputs.get(step_id).ok_or_else(|| {
            AegisError::Internal(format!(
                "Interpolation failure: step '{}' output not available in pipeline",
                step_id
            ))
        })?;

        let mut curr = root;
        for &key in &parts[1..] {
            if key == "output" || key == "result" {
                if let Some(inner) = curr.get(key) {
                    curr = inner;
                    continue;
                }
            }
            curr = curr.get(key).ok_or_else(|| {
                AegisError::Internal(format!(
                    "Property '{}' not found in output of step '{}'",
                    key, step_id
                ))
            })?;
        }

        Ok(curr.clone())
    }

    /// Execute a multi-step execution plan sequentially in topological waves
    pub async fn execute_pipeline<F, Fut>(
        &self,
        plan: &ExecutionPlan,
        step_runner: F,
    ) -> AegisResult<PipelineExecutionResult>
    where
        F: Fn(String, String, Value) -> Fut,
        Fut: Future<Output = AegisResult<Value>>,
    {
        let start_time = Instant::now();

        // 1. Topological Sort Order
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();

        for step in &plan.steps {
            in_degree.insert(&step.id, 0);
            adj.insert(&step.id, Vec::new());
        }

        for step in &plan.steps {
            for dep in &step.depends_on {
                adj.entry(dep).or_default().push(&step.id);
                *in_degree.entry(&step.id).or_default() += 1;
            }
        }

        // Kahn's algorithm queue
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut ordered_ids = Vec::new();
        while let Some(curr) = queue.pop() {
            ordered_ids.push(curr);
            if let Some(neighbors) = adj.get(curr) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        if ordered_ids.len() != plan.steps.len() {
            return Err(AegisError::Internal(
                "Cannot execute pipeline: cyclic dependency detected in plan".to_string(),
            ));
        }

        let step_map: HashMap<&str, &crate::discovery::planner::PlanStep> =
            plan.steps.iter().map(|s| (s.id.as_str(), s)).collect();

        let mut outputs: HashMap<String, Value> = HashMap::new();
        let mut step_results: Vec<PipelineStepResult> = Vec::new();
        let mut pipeline_success = true;

        for step_id in ordered_ids {
            let step = step_map[step_id];
            let step_start = Instant::now();

            // Interpolate arguments from prior step outputs
            let interpolated_args = match Self::interpolate_value(&step.arguments, &outputs) {
                Ok(args) => args,
                Err(e) => {
                    pipeline_success = false;
                    step_results.push(PipelineStepResult {
                        step_id: step.id.clone(),
                        tool: step.tool.clone(),
                        success: false,
                        output: Value::Null,
                        latency_ms: step_start.elapsed().as_millis() as u64,
                        error: Some(format!("Interpolation failed: {e}")),
                    });
                    break;
                }
            };

            // Execute the step
            match step_runner(step.id.clone(), step.tool.clone(), interpolated_args).await {
                Ok(out) => {
                    let latency = step_start.elapsed().as_millis() as u64;
                    outputs.insert(step.id.clone(), out.clone());
                    step_results.push(PipelineStepResult {
                        step_id: step.id.clone(),
                        tool: step.tool.clone(),
                        success: true,
                        output: out,
                        latency_ms: latency,
                        error: None,
                    });
                }
                Err(err) => {
                    pipeline_success = false;
                    let latency = step_start.elapsed().as_millis() as u64;
                    step_results.push(PipelineStepResult {
                        step_id: step.id.clone(),
                        tool: step.tool.clone(),
                        success: false,
                        output: Value::Null,
                        latency_ms: latency,
                        error: Some(err.to_string()),
                    });
                    break; // Fail-closed on intermediate step failure
                }
            }
        }

        let total_latency = start_time.elapsed().as_millis() as u64;
        let completed = step_results.iter().filter(|s| s.success).count();

        Ok(PipelineExecutionResult {
            plan_id: plan.plan_id.clone(),
            success: pipeline_success,
            total_steps: plan.steps.len(),
            completed_steps: completed,
            overall_risk: PlanRisk::Low,
            step_results,
            step_outputs: outputs,
            total_latency_ms: total_latency,
        })
    }
}
