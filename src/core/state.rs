// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use crate::core::error::PropylonResult;
use crate::core::types::TenantId;

/// Core distributed state abstraction.
/// Implementations may range from in-memory (dev) to Redis Cluster / PostgreSQL (production).
pub trait DistributedState: DistributedCache + DistributedRateLimiter + DistributedCircuitBreaker + QuotaEngine {}

/// Distributed caching contract for cross-pod coordination
#[async_trait]
pub trait DistributedCache: Send + Sync {
    async fn get(&self, key: &str) -> PropylonResult<Option<Value>>;
    async fn set(&self, key: &str, value: &Value, ttl: Duration) -> PropylonResult<()>;
    async fn delete(&self, key: &str) -> PropylonResult<bool>;
}

/// Distributed token bucket / sliding window rate limiting
#[async_trait]
pub trait DistributedRateLimiter: Send + Sync {
    /// Acquire rate limit quota. Returns Ok(true) if allowed, Ok(false) or Err if exceeded.
    async fn acquire(&self, tenant: &TenantId, resource: &str, tokens: u32) -> PropylonResult<bool>;
}

/// Distributed circuit breaker to prevent cascading backend failures
#[async_trait]
pub trait DistributedCircuitBreaker: Send + Sync {
    /// Check whether calls to backend are permitted (Closed or Half-Open)
    async fn is_available(&self, backend: &str) -> PropylonResult<bool>;
    /// Record successful invocation
    async fn record_success(&self, backend: &str) -> PropylonResult<()>;
    /// Record backend failure
    async fn record_failure(&self, backend: &str) -> PropylonResult<()>;
}

/// Multi-tenant cost and token quota tracking (Pillar 5)
#[async_trait]
pub trait QuotaEngine: Send + Sync {
    async fn check_budget(&self, tenant: &TenantId) -> PropylonResult<bool>;
    async fn record_spend(&self, tenant: &TenantId, cost_usd: f64) -> PropylonResult<()>;
}

/// Tenant monthly budget configuration and usage state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantBudget {
    pub tenant_id: TenantId,
    pub department: String,
    pub monthly_limit_usd: f64,
    pub current_spend_usd: f64,
    pub tokens_consumed: u64,
    pub hard_freeze_enabled: bool,
}

/// Dynamic tier status for automated warning and hard freeze
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BudgetStatus {
    Normal { percent_used: f64 },
    Warning { percent_used: f64, message: String },
    Frozen { percent_used: f64, reason: String },
}

/// Departmental chargeback rollup entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentChargeback {
    pub department: String,
    pub total_cost_usd: f64,
    pub total_tokens: u64,
    pub tenant_count: usize,
}

/// Enterprise budget management contract
#[async_trait]
pub trait BudgetManager: Send + Sync {
    async fn set_budget(&self, budget: TenantBudget) -> PropylonResult<()>;
    async fn get_budget(&self, tenant: &TenantId) -> PropylonResult<Option<TenantBudget>>;
    async fn record_usage(&self, tenant: &TenantId, tokens: u64, cost_usd: f64) -> PropylonResult<()>;
    async fn evaluate_status(&self, tenant: &TenantId) -> PropylonResult<BudgetStatus>;
    async fn generate_chargeback_report(&self) -> PropylonResult<Vec<DepartmentChargeback>>;
    async fn render_prometheus_metrics(&self) -> PropylonResult<String>;
}
