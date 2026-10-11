// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::error::PropylonResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropylonBackendCrd {
    pub name: String,
    pub namespace: String,
    pub transport: String,
    pub endpoint_or_cmd: String,
    pub replicas: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropylonPolicyCrd {
    pub name: String,
    pub namespace: String,
    pub tier: String,
    pub rate_limit_rps: u32,
    pub circuit_breaker_threshold: u32,
    pub dlp_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropylonTenantCrd {
    pub tenant_id: String,
    pub namespace: String,
    pub monthly_token_budget: u64,
    pub cost_center: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmissionDecision {
    pub allowed: bool,
    pub status_code: u16,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconcileOutcome {
    pub resource_name: String,
    pub generation: u64,
    pub status: String,
    pub message: String,
}

/// Interface-First abstraction for Kubernetes CRD Reconciliation & Admission Control
#[async_trait]
pub trait PropylonCrdReconciler: Send + Sync {
    async fn reconcile_backend(&self, spec: &PropylonBackendCrd) -> PropylonResult<ReconcileOutcome>;
    async fn reconcile_policy(&self, spec: &PropylonPolicyCrd) -> PropylonResult<ReconcileOutcome>;
    async fn reconcile_tenant(&self, spec: &PropylonTenantCrd) -> PropylonResult<ReconcileOutcome>;
    async fn validate_admission(&self, resource_kind: &str, raw_json: &serde_json::Value) -> PropylonResult<AdmissionDecision>;
    async fn mutate_pod_spec(&self, pod_spec: &serde_json::Value) -> PropylonResult<serde_json::Value>;
}
