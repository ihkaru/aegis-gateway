//! Namespaced Catalog & Collision Prevention Engine
//! Resolves docker/mcp-gateway#581
//! Ensures servers with identically named tools or prompts can coexist safely.

use std::collections::HashMap;
use crate::core::types::ToolDefinition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDefinition {
    pub name: String,
    pub description: String,
    pub server: String,
    pub arguments: Vec<String>,
}

#[derive(Default, Debug)]
pub struct NamespacedCatalog {
    tools: HashMap<String, ToolDefinition>,
    prompts: HashMap<String, PromptDefinition>,
    tool_name_occurrences: HashMap<String, Vec<String>>,
    prompt_name_occurrences: HashMap<String, Vec<String>>,
}

impl NamespacedCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn make_namespaced_id(server: &str, name: &str) -> String {
        format!("{}__{}", server, name)
    }

    pub fn register_tool(&mut self, mut tool: ToolDefinition, server: &str) -> String {
        let original_name = tool.name.clone();
        let namespaced = Self::make_namespaced_id(server, &original_name);

        self.tool_name_occurrences
            .entry(original_name.clone())
            .or_default()
            .push(server.to_string());

        // Store by namespaced key
        tool.name = namespaced.clone();
        self.tools.insert(namespaced.clone(), tool.clone());

        // If it's the first occurrence, also store by unqualified name
        let occurrences = &self.tool_name_occurrences[&original_name];
        if occurrences.len() == 1 {
            let mut unqual = tool.clone();
            unqual.name = original_name.clone();
            self.tools.insert(original_name, unqual);
        } else {
            // Collision detected! Remove ambiguous unqualified alias to prevent accidental routing
            self.tools.remove(&original_name);
        }

        namespaced
    }

    pub fn register_prompt(&mut self, mut prompt: PromptDefinition, server: &str) -> String {
        let original_name = prompt.name.clone();
        let namespaced = Self::make_namespaced_id(server, &original_name);

        self.prompt_name_occurrences
            .entry(original_name.clone())
            .or_default()
            .push(server.to_string());

        prompt.server = server.to_string();
        self.prompts.insert(namespaced.clone(), prompt.clone());

        let occurrences = &self.prompt_name_occurrences[&original_name];
        if occurrences.len() == 1 {
            self.prompts.insert(original_name, prompt);
        } else {
            // Collision detected! Remove ambiguous unqualified prompt
            self.prompts.remove(&original_name);
        }

        namespaced
    }

    pub fn resolve_tool(&self, query: &str) -> Option<&ToolDefinition> {
        self.tools.get(query)
    }

    pub fn resolve_prompt(&self, query: &str) -> Option<&PromptDefinition> {
        self.prompts.get(query)
    }

    pub fn has_tool_collision(&self, name: &str) -> bool {
        self.tool_name_occurrences
            .get(name)
            .map(|servers| servers.len() > 1)
            .unwrap_or(false)
    }

    pub fn has_prompt_collision(&self, name: &str) -> bool {
        self.prompt_name_occurrences
            .get(name)
            .map(|servers| servers.len() > 1)
            .unwrap_or(false)
    }

    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        // Return canonical namespaced tools to avoid duplicates
        self.tools
            .iter()
            .filter(|(k, _)| k.contains("__"))
            .map(|(_, v)| v)
            .collect()
    }

    pub fn list_prompts(&self) -> Vec<&PromptDefinition> {
        self.prompts
            .iter()
            .filter(|(k, _)| k.contains("__"))
            .map(|(_, v)| v)
            .collect()
    }
}
