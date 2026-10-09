// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::core::error::AegisResult;
use crate::core::state::{
    DistributedCache, DistributedCircuitBreaker, DistributedRateLimiter, DistributedState,
    QuotaEngine,
};
use crate::core::types::TenantId;

pub mod drain;
pub mod health;
pub mod quota;
pub mod redis_backend;
pub mod session_fence;

pub use drain::{DrainCoordinator, TaskGuard};
pub use health::GatewayHealthService;
pub use quota::HardFreezeQuota;
pub use redis_backend::RedisStateBackend;
pub use session_fence::InMemorySessionFence;

/// Thread-safe in-memory state backend for development & testing.
/// In production, swap with `RedisStateBackend` or `PostgreSqlStateBackend` with zero code change.
#[derive(Clone, Default)]
pub struct InMemoryStateBackend {

    cache: Arc<RwLock<HashMap<String, (Value, Instant, Duration)>>>,
    rate_buckets: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
    circuit_failures: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
    tenant_spend: Arc<RwLock<HashMap<TenantId, f64>>>,
}

impl InMemoryStateBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DistributedState for InMemoryStateBackend {}

#[async_trait]
impl DistributedCache for InMemoryStateBackend {
    async fn get(&self, key: &str) -> AegisResult<Option<Value>> {
        let read = self.cache.read().await;
        if let Some((val, inserted, ttl)) = read.get(key) {
            if inserted.elapsed() < *ttl {
                return Ok(Some(val.clone()));
            }
        }
        Ok(None)
    }

    async fn set(&self, key: &str, value: &Value, ttl: Duration) -> AegisResult<()> {
        let mut write = self.cache.write().await;
        write.insert(key.to_string(), (value.clone(), Instant::now(), ttl));
        Ok(())
    }

    async fn delete(&self, key: &str) -> AegisResult<bool> {
        let mut write = self.cache.write().await;
        Ok(write.remove(key).is_some())
    }
}

#[async_trait]
impl DistributedRateLimiter for InMemoryStateBackend {
    async fn acquire(&self, tenant: &TenantId, resource: &str, tokens: u32) -> AegisResult<bool> {
        let key = format!("{}:{}", tenant.as_str(), resource);
        let mut buckets = self.rate_buckets.write().await;
        let entry = buckets.entry(key).or_insert((100, Instant::now()));

        // Replenish bucket if more than 1 second elapsed
        if entry.1.elapsed() >= Duration::from_secs(1) {
            entry.0 = 100;
            entry.1 = Instant::now();
        }

        if entry.0 >= tokens {
            entry.0 -= tokens;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[async_trait]
impl DistributedCircuitBreaker for InMemoryStateBackend {
    async fn is_available(&self, backend: &str) -> AegisResult<bool> {
        let read = self.circuit_failures.read().await;
        if let Some((fails, last_fail)) = read.get(backend) {
            // Open if >= 5 failures within last 30 seconds
            if *fails >= 5 && last_fail.elapsed() < Duration::from_secs(30) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn record_success(&self, backend: &str) -> AegisResult<()> {
        let mut write = self.circuit_failures.write().await;
        write.remove(backend);
        Ok(())
    }

    async fn record_failure(&self, backend: &str) -> AegisResult<()> {
        let mut write = self.circuit_failures.write().await;
        let entry = write.entry(backend.to_string()).or_insert((0, Instant::now()));
        entry.0 += 1;
        entry.1 = Instant::now();
        Ok(())
    }
}

#[async_trait]
impl QuotaEngine for InMemoryStateBackend {
    async fn check_budget(&self, tenant: &TenantId) -> AegisResult<bool> {
        let read = self.tenant_spend.read().await;
        let current = read.get(tenant).copied().unwrap_or(0.0);
        // Default cap of $500 per month
        Ok(current < 500.0)
    }

    async fn record_spend(&self, tenant: &TenantId, cost_usd: f64) -> AegisResult<()> {
        let mut write = self.tenant_spend.write().await;
        let entry = write.entry(tenant.clone()).or_insert(0.0);
        *entry += cost_usd;
        Ok(())
    }
}
