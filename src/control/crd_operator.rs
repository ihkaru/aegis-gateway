// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::core::control_plane::{DynamicControlPlane, RuntimeBackendUpdate, RuntimePolicyUpdate};
use crate::core::error::{AegisError, AegisResult};

pub const EXPECTED_CRD_API_VERSION: &str = "aegis.enterprise.io/v1alpha1";

/// Declarative Kubernetes CRD manifest for Aegis Backend registration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AegisBackendCrd {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: CrdMetadata,
    pub spec: AegisBackendSpec,
}

/// Declarative Kubernetes CRD manifest for Aegis Policy configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AegisPolicyCrd {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: CrdMetadata,
    pub spec: AegisPolicySpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdMetadata {
    pub name: String,
    #[serde(default = "default_namespace")]
    pub namespace: String,
}

fn default_namespace() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AegisBackendSpec {
    pub transport: String,
    #[serde(alias = "endpointOrCmd")]
    pub endpoint_or_cmd: String,
    #[serde(default = "default_replicas")]
    pub replicas: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_replicas() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AegisPolicySpec {
    pub tier: String,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_rps: u32,
    #[serde(default = "default_circuit_breaker")]
    pub circuit_breaker_threshold: u32,
    #[serde(default = "default_true")]
    pub dlp_enabled: bool,
}

fn default_rate_limit() -> u32 {
    100
}

fn default_circuit_breaker() -> u32 {
    5
}

/// Summary report of declarative CRD reconciliation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub applied_backends: Vec<String>,
    pub applied_policies: Vec<String>,
    pub active_tier: Option<String>,
    pub control_plane_version: u64,
    pub checksum_sha256: String,
}

/// Declarative Kubernetes CRD Operator Engine
pub struct CrdOperatorEngine {
    control_plane: Arc<dyn DynamicControlPlane>,
}

impl CrdOperatorEngine {
    pub fn new(control_plane: Arc<dyn DynamicControlPlane>) -> Self {
        Self { control_plane }
    }

    /// Parse and validate multi-document Kubernetes CRD YAML manifests
    pub fn parse_and_validate(
        manifest_yaml: &str,
    ) -> AegisResult<(Vec<AegisBackendCrd>, Vec<AegisPolicyCrd>)> {
        let mut backends = Vec::new();
        let mut policies = Vec::new();

        for doc in manifest_yaml.split("\n---") {
            let doc_trimmed = doc.trim();
            if doc_trimmed.is_empty() {
                continue;
            }

            let raw_val: serde_yaml::Value = serde_yaml::from_str(doc_trimmed)
                .map_err(|e| AegisError::Validation(format!("Invalid YAML document: {e}")))?;

            let api_version = raw_val
                .get("apiVersion")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AegisError::Validation("Missing 'apiVersion' in manifest".to_string()))?;

            if api_version != EXPECTED_CRD_API_VERSION {
                return Err(AegisError::Validation(format!(
                    "Unsupported apiVersion '{api_version}', expected '{EXPECTED_CRD_API_VERSION}'"
                )));
            }

            let kind = raw_val
                .get("kind")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AegisError::Validation("Missing 'kind' in manifest".to_string()))?;

            match kind {
                "AegisBackend" => {
                    let crd: AegisBackendCrd = serde_yaml::from_value(raw_val)
                        .map_err(|e| AegisError::Validation(format!("Invalid AegisBackend schema: {e}")))?;

                    let transport = crd.spec.transport.to_lowercase();
                    if transport != "stdio" && transport != "sse" && transport != "http" {
                        return Err(AegisError::Validation(format!(
                            "Invalid transport '{}' in backend '{}': expected stdio, sse, or http",
                            crd.spec.transport, crd.metadata.name
                        )));
                    }

                    if crd.spec.endpoint_or_cmd.trim().is_empty() {
                        return Err(AegisError::Validation(format!(
                            "Backend '{}' specifies empty endpoint_or_cmd",
                            crd.metadata.name
                        )));
                    }

                    backends.push(crd);
                }
                "AegisPolicy" => {
                    let crd: AegisPolicyCrd = serde_yaml::from_value(raw_val)
                        .map_err(|e| AegisError::Validation(format!("Invalid AegisPolicy schema: {e}")))?;

                    let tier = crd.spec.tier.to_lowercase();
                    if tier != "dev" && tier != "hybrid" && tier != "strict" {
                        return Err(AegisError::Validation(format!(
                            "Invalid policy tier '{}' in policy '{}': expected dev, hybrid, or strict",
                            crd.spec.tier, crd.metadata.name
                        )));
                    }

                    if crd.spec.rate_limit_rps == 0 {
                        return Err(AegisError::Validation(format!(
                            "Policy '{}' specifies rate_limit_rps = 0 (must be > 0)",
                            crd.metadata.name
                        )));
                    }

                    policies.push(crd);
                }
                other => {
                    return Err(AegisError::Validation(format!(
                        "Unsupported CRD kind '{other}': expected AegisBackend or AegisPolicy"
                    )));
                }
            }
        }

        Ok((backends, policies))
    }

    /// Reconcile declarative CRD manifest with live runtime control plane
    pub async fn reconcile(&self, manifest_yaml: &str) -> AegisResult<ReconciliationReport> {
        let (backends, policies) = Self::parse_and_validate(manifest_yaml)?;
        let mut applied_backends = Vec::new();
        let mut applied_policies = Vec::new();
        let mut last_version = 0;
        let mut active_tier = None;

        for b in backends {
            let update = RuntimeBackendUpdate {
                backend_name: b.metadata.name.clone(),
                transport: b.spec.transport.clone(),
                endpoint_or_cmd: b.spec.endpoint_or_cmd.clone(),
                enabled: b.spec.enabled,
            };
            last_version = self.control_plane.apply_backend_update(&update).await?;
            applied_backends.push(b.metadata.name);
        }

        for p in policies {
            let tier = p.spec.tier.clone();
            let update = RuntimePolicyUpdate {
                tenant_id: p.metadata.name.clone(),
                policy_tier: tier.clone(),
                rate_limit_rps: p.spec.rate_limit_rps,
                dlp_enabled: p.spec.dlp_enabled,
            };
            last_version = self.control_plane.apply_policy_update(&update).await?;
            active_tier = Some(tier);
            applied_policies.push(p.metadata.name);
        }

        let mut hasher = Sha256::new();
        hasher.update(manifest_yaml.as_bytes());
        let result = hasher.finalize();
        let checksum_sha256 = result.iter().map(|b| format!("{:02x}", b)).collect();

        Ok(ReconciliationReport {
            applied_backends,
            applied_policies,
            active_tier,
            control_plane_version: last_version,
            checksum_sha256,
        })
    }
}
