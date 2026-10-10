// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::error::AegisResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeBackendUpdate {
    pub backend_name: String,
    pub transport: String,
    pub endpoint_or_cmd: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePolicyUpdate {
    pub tenant_id: String,
    pub policy_tier: String,
    pub rate_limit_rps: u32,
    pub dlp_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DynamicControlPlaneStatus {
    pub active_backends_count: usize,
    pub active_policies_count: usize,
    pub last_reload_unix: u64,
    pub config_version: u64,
}

/// Interface-First abstraction for Dynamic Runtime Control Plane
#[async_trait]
pub trait DynamicControlPlane: Send + Sync {
    async fn apply_backend_update(&self, update: &RuntimeBackendUpdate) -> AegisResult<u64>;
    async fn remove_backend(&self, backend_name: &str) -> AegisResult<bool>;
    async fn apply_policy_update(&self, update: &RuntimePolicyUpdate) -> AegisResult<u64>;
    async fn get_status(&self) -> AegisResult<DynamicControlPlaneStatus>;
}

/// Interface-First abstraction for File/Signal-driven Hot-Reload
#[async_trait]
pub trait ConfigurationWatcher: Send + Sync {
    async fn trigger_reload(&self) -> AegisResult<u64>;
    async fn watch_signal(&self) -> AegisResult<()>;
}
