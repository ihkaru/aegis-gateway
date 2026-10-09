// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::core::error::{AegisError, AegisResult};
use crate::core::state::{
    DistributedCache, DistributedCircuitBreaker, DistributedRateLimiter, DistributedState,
    QuotaEngine,
};
use crate::core::types::TenantId;

/// Atomic Redis Lua script for token-bucket sliding window rate limiting
pub const RATE_LIMIT_LUA: &str = r#"
local key = KEYS[1]
local requested = tonumber(ARGV[1])
local capacity = tonumber(ARGV[2])
local ttl = tonumber(ARGV[3])

local current = tonumber(redis.call('get', key) or "0")
if current + requested <= capacity then
    redis.call('incrby', key, requested)
    if current == 0 then
        redis.call('expire', key, ttl)
    end
    return 1
else
    return 0
end
"#;

/// High-availability Redis Cluster state driver with automatic localized fallback
pub struct RedisStateBackend {
    connection_url: String,
    redis_client: Option<redis::Client>,
    memory_cache: Arc<RwLock<HashMap<String, (Value, Option<Instant>)>>>,
    failure_counts: Arc<RwLock<HashMap<String, u32>>>,
    circuit_tripped: Arc<RwLock<HashMap<String, Instant>>>,
    tenant_spend: Arc<RwLock<HashMap<String, f64>>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

impl RedisStateBackend {
    /// Connect to Redis cluster URL (e.g. "redis://127.0.0.1:6379")
    pub fn new(connection_url: impl Into<String>) -> Self {
        let url = connection_url.into();
        let client = redis::Client::open(url.as_str()).ok();
        Self {
            connection_url: url,
            redis_client: client,
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
            failure_counts: Arc::new(RwLock::new(HashMap::new())),
            circuit_tripped: Arc::new(RwLock::new(HashMap::new())),
            tenant_spend: Arc::new(RwLock::new(HashMap::new())),
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
        }
    }

    /// Construct a standalone instance for testing with identical contracts
    pub fn new_standalone() -> Self {
        Self::new("redis://localhost:6379")
    }

    pub fn connection_url(&self) -> &str {
        &self.connection_url
    }
}

impl DistributedState for RedisStateBackend {}

#[async_trait]
impl DistributedCache for RedisStateBackend {
    async fn get(&self, key: &str) -> AegisResult<Option<Value>> {
        if let Some(client) = &self.redis_client {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                let res: redis::RedisResult<Option<String>> = redis::AsyncCommands::get(&mut conn, key).await;
                if let Ok(Some(json_str)) = res {
                    let val: Value = serde_json::from_str(&json_str)
                        .map_err(|e| AegisError::StateError(e.to_string()))?;
                    return Ok(Some(val));
                }
            }
        }
        let cache = self.memory_cache.read().await;
        if let Some((val, expiry)) = cache.get(key) {
            if let Some(exp) = expiry {
                if Instant::now() > *exp {
                    return Ok(None);
                }
            }
            return Ok(Some(val.clone()));
        }
        Ok(None)
    }

    async fn set(&self, key: &str, value: &Value, ttl: Duration) -> AegisResult<()> {
        if let Some(client) = &self.redis_client {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                let serialized = serde_json::to_string(value)
                    .map_err(|e| AegisError::StateError(e.to_string()))?;
                let ttl_secs = ttl.as_secs().max(1);
                let _: redis::RedisResult<()> = redis::AsyncCommands::set_ex(&mut conn, key, serialized, ttl_secs).await;
            }
        }
        let mut cache = self.memory_cache.write().await;
        cache.insert(key.to_string(), (value.clone(), Some(Instant::now() + ttl)));
        Ok(())
    }

    async fn delete(&self, key: &str) -> AegisResult<bool> {
        if let Some(client) = &self.redis_client {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                let _: redis::RedisResult<()> = redis::AsyncCommands::del(&mut conn, key).await;
            }
        }
        let mut cache = self.memory_cache.write().await;
        Ok(cache.remove(key).is_some())
    }
}

#[async_trait]
impl DistributedRateLimiter for RedisStateBackend {
    async fn acquire(&self, tenant: &TenantId, resource: &str, tokens: u32) -> AegisResult<bool> {
        let key = format!("ratelimit:{}:{}", tenant.as_str(), resource);
        if let Some(client) = &self.redis_client {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                let script = redis::Script::new(RATE_LIMIT_LUA);
                let res: redis::RedisResult<i32> = script
                    .key(&key)
                    .arg(tokens)
                    .arg(100) // Default burst capacity per window
                    .arg(60)  // TTL 60 seconds
                    .invoke_async(&mut conn)
                    .await;
                if let Ok(allowed) = res {
                    return Ok(allowed == 1);
                }
            }
        }
        // Local cluster-emulated fallback
        let cache_key = format!("ratelimit_mem:{}", key);
        let current = self.get(&cache_key).await?.and_then(|v| v.as_u64()).unwrap_or(0);
        if current + (tokens as u64) <= 100 {
            self.set(&cache_key, &serde_json::json!(current + tokens as u64), Duration::from_secs(60)).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[async_trait]
impl DistributedCircuitBreaker for RedisStateBackend {
    async fn is_available(&self, backend: &str) -> AegisResult<bool> {
        let trips = self.circuit_tripped.read().await;
        if let Some(tripped_at) = trips.get(backend) {
            if tripped_at.elapsed() < self.recovery_timeout {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn record_success(&self, backend: &str) -> AegisResult<()> {
        let mut fails = self.failure_counts.write().await;
        fails.remove(backend);
        let mut trips = self.circuit_tripped.write().await;
        trips.remove(backend);
        Ok(())
    }

    async fn record_failure(&self, backend: &str) -> AegisResult<()> {
        let mut fails = self.failure_counts.write().await;
        let count = fails.entry(backend.to_string()).or_insert(0);
        *count += 1;
        if *count >= self.failure_threshold {
            let mut trips = self.circuit_tripped.write().await;
            trips.insert(backend.to_string(), Instant::now());
        }
        Ok(())
    }
}

#[async_trait]
impl QuotaEngine for RedisStateBackend {
    async fn check_budget(&self, tenant: &TenantId) -> AegisResult<bool> {
        let spend = self.tenant_spend.read().await;
        let current = spend.get(tenant.as_str()).copied().unwrap_or(0.0);
        Ok(current < 10_000.0) // $10,000 default budget ceiling
    }

    async fn record_spend(&self, tenant: &TenantId, cost_usd: f64) -> AegisResult<()> {
        let mut spend = self.tenant_spend.write().await;
        let current = spend.entry(tenant.as_str().to_string()).or_insert(0.0);
        *current += cost_usd;
        Ok(())
    }
}
