// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::approval::{ApprovalDecision, ApprovalGate};
use crate::core::data_governance::{
    DataClassification, DataEgressDecision, DataEgressPolicyEngine, PolicyTier,
};
use crate::core::error::AegisResult;
use crate::core::types::CallerContext;

/// Configurable, multi-tiered enterprise data egress policy engine
pub struct TieredDataEgressEngine {
    active_tier: Arc<RwLock<PolicyTier>>,
    max_direct_egress_bytes: u64,
    approval_gate: Option<Arc<dyn ApprovalGate>>,
}

impl TieredDataEgressEngine {
    pub fn new(
        initial_tier: PolicyTier,
        max_direct_egress_bytes: u64,
        approval_gate: Option<Arc<dyn ApprovalGate>>,
    ) -> Self {
        Self {
            active_tier: Arc::new(RwLock::new(initial_tier)),
            max_direct_egress_bytes,
            approval_gate,
        }
    }

    pub async fn set_tier(&self, tier: PolicyTier) {
        let mut guard = self.active_tier.write().await;
        *guard = tier;
    }

    pub async fn get_tier(&self) -> PolicyTier {
        *self.active_tier.read().await
    }
}

#[async_trait]
impl DataEgressPolicyEngine for TieredDataEgressEngine {
    async fn evaluate_egress(
        &self,
        resource_id: &str,
        resource_size_bytes: u64,
        _mime_type: &str,
        classification: DataClassification,
        caller: &CallerContext,
    ) -> AegisResult<DataEgressDecision> {
        let tier = *self.active_tier.read().await;

        // 1. Developer tier: completely frictionless, full egress allowed
        if tier == PolicyTier::Developer {
            return Ok(DataEgressDecision::AllowDirectEgress);
        }

        // 2. Derived artifacts and public resources are always safe for egress
        if classification == DataClassification::DerivedArtifact
            || classification == DataClassification::PublicResource
        {
            return Ok(DataEgressDecision::AllowDirectEgress);
        }

        // 3. Strict Air-Gapped tier: Zero-Egress on raw restricted data
        if tier == PolicyTier::StrictAirgapped {
            return Ok(DataEgressDecision::RequireInSituCompute {
                reason: format!(
                    "Strict air-gapped policy strictly forbids downloading raw dataset '{}' ({} bytes). Compute in-situ is required.",
                    resource_id, resource_size_bytes
                ),
                alternatives: vec![
                    "In-situ analytical query via DuckDB/Polars directly in the secure server enclave.".to_string(),
                    "Synthesize a derived aggregation table or summary chart without downloading raw data.".to_string(),
                ],
            });
        }

        // 4. Hybrid tier: Small files pass; large files require in-situ or approval
        if resource_size_bytes <= self.max_direct_egress_bytes {
            return Ok(DataEgressDecision::AllowDirectEgress);
        }

        // Resource exceeds hybrid threshold; generate Human-In-The-Loop approval ticket if gate configured
        if let Some(ref gate) = self.approval_gate {
            let session_id = uuid::Uuid::new_v4().to_string();
            let action_payload = serde_json::json!({
                "action": "egress_raw_dataset",
                "resource_id": resource_id,
                "size_bytes": resource_size_bytes,
                "classification": "RawRestricted",
            });

            match gate.evaluate_action(&caller.subject, &session_id, "data_egress", &action_payload).await? {
                ApprovalDecision::RequireApproval(ticket) => {
                    return Ok(DataEgressDecision::RequireApproval {
                        ticket,
                        reason: format!(
                            "Raw dataset '{}' ({} bytes) exceeds direct egress threshold ({} bytes). Managerial approval required.",
                            resource_id, resource_size_bytes, self.max_direct_egress_bytes
                        ),
                    });
                }
                ApprovalDecision::Deny { reason } => {
                    return Ok(DataEgressDecision::Deny { reason });
                }
                ApprovalDecision::AutoApprove => {
                    return Ok(DataEgressDecision::AllowDirectEgress);
                }
            }
        }

        // Fallback if no approval gate is wired
        Ok(DataEgressDecision::RequireInSituCompute {
            reason: format!(
                "Raw dataset '{}' exceeds threshold ({} bytes). Direct egress disabled.",
                resource_id, self.max_direct_egress_bytes
            ),
            alternatives: vec![
                "Process data in-situ via InSituDataEnclave".to_string(),
            ],
        })
    }

    fn current_tier(&self) -> PolicyTier {
        // Safe default reading without blocking async callers
        PolicyTier::Hybrid
    }
}
