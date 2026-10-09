// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::core::error::AegisResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBundle {
    pub metadata: SkillMetadata,
    pub instructions_markdown: String,
    pub auxiliary_files: Vec<(String, String)>,
}

/// Centralized skill registry trait with progressive on-demand loading
#[async_trait]
pub trait SkillRegistry: Send + Sync {
    /// List available skills (metadata only — lightweight Tier 1)
    async fn list_skills(&self) -> AegisResult<Vec<SkillMetadata>>;

    /// Load full skill instructions on-demand (Tier 2 activation)
    async fn load_skill(&self, name: &str) -> AegisResult<SkillBundle>;

    /// Refresh skills from disk / remote repo (hot-reload)
    async fn reload(&self) -> AegisResult<usize>;
}

/// Active scanner for tool/skill descriptions against prompt injection & tool poisoning
pub trait PoisonScanner: Send + Sync {
    fn detect_injection(&self, text: &str) -> AegisResult<()>;
}
