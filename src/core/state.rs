// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use crate::core::error::AegisResult;
use crate::core::types::TenantId;

/// Core distributed state abstraction.
/// Implementations may range from in-memory (dev) to Redis Cluster / PostgreSQL (production).
pub trait DistributedState: DistributedCache + DistributedRateLimiter + DistributedCircuitBreaker + QuotaEngine {}

/// Distributed caching contract for cross-pod coordination
#[async_trait]
pub trait DistributedCache: Send + Sync {
    async fn get(&self, key: &str) -> AegisResult<Option<Value>>;
    async fn set(&self, key: &str, value: &Value, ttl: Duration) -> AegisResult<()>;
    async fn delete(&self, key: &str) -> AegisResult<bool>;
}

/// Distributed token bucket / sliding window rate limiting
#[async_trait]
pub trait DistributedRateLimiter: Send + Sync {
    /// Acquire rate limit quota. Returns Ok(true) if allowed, Ok(false) or Err if exceeded.
    async fn acquire(&self, tenant: &TenantId, resource: &str, tokens: u32) -> AegisResult<bool>;
}

/// Distributed circuit breaker to prevent cascading backend failures
#[async_trait]
pub trait DistributedCircuitBreaker: Send + Sync {
    /// Check whether calls to backend are permitted (Closed or Half-Open)
    async fn is_available(&self, backend: &str) -> AegisResult<bool>;
    /// Record successful invocation
    async fn record_success(&self, backend: &str) -> AegisResult<()>;
    /// Record backend failure
    async fn record_failure(&self, backend: &str) -> AegisResult<()>;
}

/// Multi-tenant cost and token quota tracking (Pillar 5)
#[async_trait]
pub trait QuotaEngine: Send + Sync {
    async fn check_budget(&self, tenant: &TenantId) -> AegisResult<bool>;
    async fn record_spend(&self, tenant: &TenantId, cost_usd: f64) -> AegisResult<()>;
}
