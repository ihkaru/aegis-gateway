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

/// Skill SemVer and tool dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub skill_name: String,
    pub semver_req: String,
    pub required_tools: Vec<String>,
}

/// Signed skill bundle with cryptographic provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedSkillBundle {
    pub bundle: SkillBundle,
    pub sha256_digest: String,
    pub signature: Option<String>,
    pub signer_identity: Option<String>,
}

/// GitOps sync summary report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsSyncReport {
    pub synced_count: usize,
    pub updated_skills: Vec<String>,
    pub verified_signatures: usize,
    pub errors: Vec<String>,
}

/// Semantic skill vector retriever for natural language discovery without prompt stuffing
#[async_trait]
pub trait SkillVectorRetriever: Send + Sync {
    async fn search_skills(&self, query_prompt: &str, top_k: usize) -> AegisResult<Vec<SkillMetadata>>;
}

