// SPDX-License-Identifier: MIT

use crate::core::types::{DisclosureTier, ProjectedTool, ToolDefinition};

pub mod catalog_index;
pub mod hybrid_search;
pub mod namespace;
pub mod pipeline;
pub mod planner;
pub mod projector;

pub use catalog_index::{
    BackendCatalogMetadata, BackendIndexStatus, CatalogSearchIndex, CatalogSearchResult,
};
pub use hybrid_search::{HybridSearchEngine, HybridSearchResult};
pub use namespace::{NamespacedCatalog, PromptDefinition};
pub use pipeline::{DagPipelineEngine, PipelineExecutionResult, PipelineStepResult};
pub use planner::{
    ExecutionPlan, ExecutionPlanner, PlanRisk, PlanStep, PlanStepValidation, PlanValidation,
};
pub use projector::ProgressiveProjector;

/// Project a full tool definition into an optimized progressive disclosure tier
pub fn project_tool(tool: &ToolDefinition, tier: DisclosureTier, score: f64) -> ProjectedTool {
    let summary = if tool.description.len() > 120 {
        format!("{}...", &tool.description[..117])
    } else {
        tool.description.clone()
    };

    match tier {
        DisclosureTier::L0 => ProjectedTool {
            name: tool.name.clone(),
            server: tool.server.clone(),
            tier: DisclosureTier::L0,
            summary,
            signature: None,
            required_params: None,
            when_to_use: None,
            input_schema: None,
            score,
        },
        DisclosureTier::L1 => ProjectedTool {
            name: tool.name.clone(),
            server: tool.server.clone(),
            tier: DisclosureTier::L1,
            summary,
            signature: Some(format!("{}({})", tool.name, tool.required_params.join(", "))),
            required_params: Some(tool.required_params.clone()),
            when_to_use: Some(tool.when_to_use.clone()),
            input_schema: None,
            score,
        },
        DisclosureTier::L2 => ProjectedTool {
            name: tool.name.clone(),
            server: tool.server.clone(),
            tier: DisclosureTier::L2,
            summary,
            signature: Some(format!("{}({})", tool.name, tool.required_params.join(", "))),
            required_params: Some(tool.required_params.clone()),
            when_to_use: Some(tool.when_to_use.clone()),
            input_schema: Some(tool.input_schema.clone()),
            score,
        },
    }
}
