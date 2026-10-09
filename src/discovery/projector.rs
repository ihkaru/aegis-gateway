// SPDX-License-Identifier: MIT

use crate::core::types::{
    DisclosureTier, ProjectedTool, ProgressiveDisclosure, TokenSavings, ToolDefinition,
};

/// Progressive disclosure engine delivering tiered tool projection and token compaction
#[derive(Clone, Default)]
pub struct ProgressiveProjector;

impl ProgressiveProjector {
    pub fn new() -> Self {
        Self
    }

    /// Estimate approximate token count using the canonical character-heuristic (4 chars ~ 1 token)
    pub fn estimate_tokens(text: &str) -> usize {
        (text.len() + 3) / 4
    }

    /// Estimate full tool definition token weight
    pub fn estimate_tool_tokens(tool: &ToolDefinition) -> usize {
        let schema_str = tool.input_schema.to_string();
        Self::estimate_tokens(&tool.name)
            + Self::estimate_tokens(&tool.server)
            + Self::estimate_tokens(&tool.description)
            + Self::estimate_tokens(&schema_str)
            + Self::estimate_tokens(&tool.when_to_use)
            + 15 // Structural overhead
    }

    /// Estimate projected tool token weight
    pub fn estimate_projected_tokens(projected: &ProjectedTool) -> usize {
        let mut count = Self::estimate_tokens(&projected.name)
            + Self::estimate_tokens(&projected.server)
            + Self::estimate_tokens(&projected.summary)
            + 8;

        if let Some(ref sig) = projected.signature {
            count += Self::estimate_tokens(sig);
        }
        if let Some(ref when) = projected.when_to_use {
            count += Self::estimate_tokens(when);
        }
        if let Some(ref schema) = projected.input_schema {
            count += Self::estimate_tokens(&schema.to_string());
        }
        count
    }

    /// Filter and rank tools according to context keywords
    pub fn filter_by_context(
        &self,
        tools: &[ToolDefinition],
        context_prompt: &str,
        tier: DisclosureTier,
        top_k: usize,
    ) -> Vec<ProjectedTool> {
        let query_lower = context_prompt.to_lowercase();
        let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f64, &ToolDefinition)> = tools
            .iter()
            .map(|t| {
                let name_lower = t.name.to_lowercase();
                let desc_lower = t.description.to_lowercase();
                let server_lower = t.server.to_lowercase();
                let mut score = 0.1; // Base score

                for q in &query_tokens {
                    if name_lower.contains(q) {
                        score += 3.0;
                    }
                    if server_lower.contains(q) {
                        score += 2.5;
                    }
                    if desc_lower.contains(q) {
                        score += 1.0;
                    }
                    for tag in &t.tags {
                        if tag.to_lowercase().contains(q) {
                            score += 2.0;
                        }
                    }
                }
                (score, t)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(top_k)
            .map(|(score, tool)| self.project(tool, tier, score))
            .collect()
    }
}

impl ProgressiveDisclosure for ProgressiveProjector {
    fn project(&self, tool: &ToolDefinition, tier: DisclosureTier, score: f64) -> ProjectedTool {
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

    fn calculate_savings(&self, original: &[ToolDefinition], tier: DisclosureTier) -> TokenSavings {
        let full_tokens: usize = original.iter().map(Self::estimate_tool_tokens).sum();

        let projected_tokens: usize = original
            .iter()
            .map(|t| {
                let p = self.project(t, tier, 1.0);
                Self::estimate_projected_tokens(&p)
            })
            .sum();

        let saved = if full_tokens > projected_tokens {
            full_tokens - projected_tokens
        } else {
            0
        };

        let reduction_percentage = if full_tokens > 0 {
            (saved as f64 / full_tokens as f64) * 100.0
        } else {
            0.0
        };

        TokenSavings {
            full_schema_tokens: full_tokens,
            projected_tokens,
            saved_tokens: saved,
            reduction_percentage,
        }
    }
}
