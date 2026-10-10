// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use serde_json::json;

use crate::core::error::{AegisError, AegisResult};
use crate::core::k8s::{
    AdmissionDecision, AegisBackendCrd, AegisCrdReconciler, AegisPolicyCrd, AegisTenantCrd,
    ReconcileOutcome,
};

/// Production Kubernetes CRD Controller & Sidecar Mutator
pub struct DeclarativeCrdController {
    backends: Arc<RwLock<HashMap<String, AegisBackendCrd>>>,
    policies: Arc<RwLock<HashMap<String, AegisPolicyCrd>>>,
    tenants: Arc<RwLock<HashMap<String, AegisTenantCrd>>>,
    proxy_image: String,
}

impl DeclarativeCrdController {
    pub fn new(proxy_image: impl Into<String>) -> Self {
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            tenants: Arc::new(RwLock::new(HashMap::new())),
            proxy_image: proxy_image.into(),
        }
    }
}

#[async_trait]
impl AegisCrdReconciler for DeclarativeCrdController {
    async fn reconcile_backend(&self, spec: &AegisBackendCrd) -> AegisResult<ReconcileOutcome> {
        if spec.endpoint_or_cmd.is_empty() {
            return Err(AegisError::Validation("Backend endpoint_or_cmd cannot be empty".into()));
        }

        let key = format!("{}/{}", spec.namespace, spec.name);
        let mut lock = self.backends.write().await;
        let is_update = lock.contains_key(&key);
        lock.insert(key.clone(), spec.clone());

        Ok(ReconcileOutcome {
            resource_name: spec.name.clone(),
            generation: 1,
            status: if is_update { "Updated".into() } else { "Applied".into() },
            message: format!("Backend {} reconciled with transport {}", spec.name, spec.transport),
        })
    }

    async fn reconcile_policy(&self, spec: &AegisPolicyCrd) -> AegisResult<ReconcileOutcome> {
        let tier = spec.tier.to_lowercase();
        if tier != "dev" && tier != "hybrid" && tier != "strict" {
            return Err(AegisError::Validation(format!(
                "Invalid policy tier '{tier}': expected dev, hybrid, or strict"
            )));
        }

        let key = format!("{}/{}", spec.namespace, spec.name);
        let mut lock = self.policies.write().await;
        let is_update = lock.contains_key(&key);
        lock.insert(key, spec.clone());

        Ok(ReconcileOutcome {
            resource_name: spec.name.clone(),
            generation: 1,
            status: if is_update { "Updated".into() } else { "Applied".into() },
            message: format!("Policy {} reconciled with tier {}", spec.name, spec.tier),
        })
    }

    async fn reconcile_tenant(&self, spec: &AegisTenantCrd) -> AegisResult<ReconcileOutcome> {
        if spec.monthly_token_budget == 0 {
            return Err(AegisError::Validation("Tenant monthly token budget must be > 0".into()));
        }

        let key = format!("{}/{}", spec.namespace, spec.tenant_id);
        let mut lock = self.tenants.write().await;
        let is_update = lock.contains_key(&key);
        lock.insert(key, spec.clone());

        Ok(ReconcileOutcome {
            resource_name: spec.tenant_id.clone(),
            generation: 1,
            status: if is_update { "Updated".into() } else { "Applied".into() },
            message: format!("Tenant {} reconciled with budget {}", spec.tenant_id, spec.monthly_token_budget),
        })
    }

    async fn validate_admission(&self, resource_kind: &str, raw_json: &serde_json::Value) -> AegisResult<AdmissionDecision> {
        match resource_kind {
            "AegisBackend" => {
                let transport = raw_json["spec"]["transport"].as_str().unwrap_or("");
                if transport != "stdio" && transport != "sse" && transport != "http" {
                    return Ok(AdmissionDecision {
                        allowed: false,
                        status_code: 422,
                        reason: Some(format!("Unsupported transport '{transport}'")),
                    });
                }
                let endpoint = raw_json["spec"]["endpoint_or_cmd"].as_str().unwrap_or("");
                if endpoint.is_empty() {
                    return Ok(AdmissionDecision {
                        allowed: false,
                        status_code: 422,
                        reason: Some("endpoint_or_cmd must not be empty".into()),
                    });
                }
                Ok(AdmissionDecision {
                    allowed: true,
                    status_code: 200,
                    reason: None,
                })
            }
            "AegisPolicy" => {
                let tier = raw_json["spec"]["tier"].as_str().unwrap_or("").to_lowercase();
                if tier != "dev" && tier != "hybrid" && tier != "strict" {
                    return Ok(AdmissionDecision {
                        allowed: false,
                        status_code: 422,
                        reason: Some(format!("Invalid tier '{tier}'")),
                    });
                }
                Ok(AdmissionDecision {
                    allowed: true,
                    status_code: 200,
                    reason: None,
                })
            }
            _ => Ok(AdmissionDecision {
                allowed: true,
                status_code: 200,
                reason: None,
            }),
        }
    }

    async fn mutate_pod_spec(&self, pod_spec: &serde_json::Value) -> AegisResult<serde_json::Value> {
        let mut mutated = pod_spec.clone();
        let containers = mutated["spec"]["containers"]
            .as_array_mut()
            .ok_or_else(|| AegisError::Validation("Invalid Pod spec: missing containers array".into()))?;

        // Sidecar injection: Add Aegis Gateway proxy sidecar
        let sidecar = json!({
            "name": "aegis-sidecar-proxy",
            "image": self.proxy_image,
            "env": [
                {
                    "name": "AEGIS_GATEWAY_URL",
                    "value": "http://aegis-gateway.aegis-system.svc.cluster.local:8080"
                },
                {
                    "name": "AEGIS_INTERCEPT_ALL_MCP",
                    "value": "true"
                }
            ],
            "resources": {
                "limits": { "cpu": "200m", "memory": "256Mi" },
                "requests": { "cpu": "50m", "memory": "64Mi" }
            }
        });

        // Avoid duplicate injection
        let already_injected = containers.iter().any(|c| c["name"] == "aegis-sidecar-proxy");
        if !already_injected {
            containers.push(sidecar);
        }

        Ok(mutated)
    }
}
