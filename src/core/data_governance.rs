// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::core::approval::ApprovalTicket;
use crate::core::error::AegisResult;
use crate::core::types::CallerContext;

/// Configurable enterprise policy strictness tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyTier {
    Developer,
    Hybrid,
    StrictAirgapped,
}

impl std::str::FromStr for PolicyTier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "dev" | "developer" => Ok(PolicyTier::Developer),
            "hybrid" => Ok(PolicyTier::Hybrid),
            "strict" | "strict_airgapped" | "airgapped" => Ok(PolicyTier::StrictAirgapped),
            other => Err(format!("Unknown policy tier: {other}")),
        }
    }
}

/// Data taxonomy classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataClassification {
    RawRestricted,
    DerivedArtifact,
    PublicResource,
}

/// Evaluation decision regarding data egress vs in-situ compute
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataEgressDecision {
    AllowDirectEgress,
    RequireInSituCompute {
        reason: String,
        alternatives: Vec<String>,
    },
    RequireApproval {
        ticket: ApprovalTicket,
        reason: String,
    },
    Deny {
        reason: String,
    },
}

/// Abstract contract for evaluating egress against enterprise posture tiers
#[async_trait]
pub trait DataEgressPolicyEngine: Send + Sync {
    /// Evaluate resource download/egress request
    async fn evaluate_egress(
        &self,
        resource_id: &str,
        resource_size_bytes: u64,
        mime_type: &str,
        classification: DataClassification,
        caller: &CallerContext,
    ) -> AegisResult<DataEgressDecision>;

    /// Return the currently active policy strictness tier
    fn current_tier(&self) -> PolicyTier;
}

/// Analytical query execution result inside server enclave
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InSituExecutionResult {
    pub success: bool,
    pub row_count: usize,
    pub columns: Vec<String>,
    pub summary_table: String,
    pub duration_ms: u64,
    pub raw_bytes_read: u64,
    pub egress_bytes_returned: u64,
}

/// Abstract contract for sandboxed in-situ data processing (Zero-Egress Analytics)
#[async_trait]
pub trait InSituDataEnclave: Send + Sync {
    /// Execute analytical transformation (e.g. SQL/DuckDB/Polars) without egressing raw data
    async fn execute_query(
        &self,
        query: &str,
        resource_path: &str,
        caller: &CallerContext,
    ) -> AegisResult<InSituExecutionResult>;
}
