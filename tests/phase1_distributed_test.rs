// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::time::Duration;
use aegis_gateway::core::health::ProbeState;

use aegis_gateway::core::state::{
    DistributedCache, DistributedCircuitBreaker, DistributedRateLimiter, QuotaEngine,
};
use aegis_gateway::core::types::TenantId;
use aegis_gateway::state::{DrainCoordinator, GatewayHealthService, RedisStateBackend};

#[tokio::test]
async fn test_phase1_redis_state_backend_contracts() {
    let backend = Arc::new(RedisStateBackend::new_standalone());
    let tenant = TenantId::new("fintech-prod");

    // 1. Distributed Cache with TTL
    let cache_key = "tool_cache:stripe:charge_1";
    let cache_val = serde_json::json!({ "status": "succeeded", "amount": 4200 });
    backend
        .set(cache_key, &cache_val, Duration::from_secs(60))
        .await
        .expect("Cache set must succeed");

    let retrieved = backend.get(cache_key).await.expect("Cache get must succeed");
    assert_eq!(retrieved, Some(cache_val));

    let deleted = backend.delete(cache_key).await.expect("Cache delete must succeed");
    assert!(deleted);
    let after_del = backend.get(cache_key).await.expect("Cache get must succeed");
    assert_eq!(after_del, None);

    // 2. Distributed Rate Limiter
    let allowed_first = backend
        .acquire(&tenant, "stripe_charge", 10)
        .await
        .expect("Rate limit check must succeed");
    assert!(allowed_first);

    // 3. Distributed Circuit Breaker
    let service = "database-shard-2";
    assert!(backend.is_available(service).await.unwrap());

    // Record 5 failures to trip the circuit
    for _ in 0..5 {
        backend.record_failure(service).await.unwrap();
    }
    assert!(
        !backend.is_available(service).await.unwrap(),
        "Circuit must be OPEN after 5 failures"
    );

    // Success recovery
    backend.record_success(service).await.unwrap();
    assert!(
        backend.is_available(service).await.unwrap(),
        "Circuit must recover to CLOSED after success"
    );

    // 4. Quota Engine
    assert!(backend.check_budget(&tenant).await.unwrap());
    backend.record_spend(&tenant, 150.75).await.unwrap();
}

#[tokio::test]
async fn test_phase1_k8s_health_and_readiness_probes() {
    let backend = Arc::new(RedisStateBackend::new_standalone());
    let health = GatewayHealthService::new(backend);

    // Liveness Probe (/healthz)
    let (liveness_code, liveness_report) = health.healthz().await;
    assert_eq!(liveness_code, 200);
    assert_eq!(liveness_report.state, ProbeState::Healthy);

    // Readiness Probe (/readyz)
    let (ready_code, ready_report) = health.readyz().await;
    assert_eq!(ready_code, 200);
    assert_eq!(ready_report.state, ProbeState::Healthy);
    assert_eq!(
        ready_report.details.get("distributed_state").map(|s| s.as_str()),
        Some("accessible")
    );
}

#[tokio::test]
async fn test_phase1_drain_coordinator_graceful_shutdown() {
    let coordinator = Arc::new(DrainCoordinator::new());
    assert_eq!(coordinator.inflight_count(), 0);

    // Acquire task slot
    let slot1 = coordinator.acquire_slot().expect("Slot 1 acquired");
    assert_eq!(coordinator.inflight_count(), 1);

    {
        let _slot2 = coordinator.acquire_slot().expect("Slot 2 acquired");
        assert_eq!(coordinator.inflight_count(), 2);
    } // slot2 drops here

    assert_eq!(coordinator.inflight_count(), 1);

    // Trigger shutdown signal
    coordinator.signal_shutdown();
    assert!(coordinator.is_draining());

    // New task requests must be rejected
    let rejected = coordinator.acquire_slot();
    assert!(rejected.is_err());

    // Background drain task
    let coord_clone = Arc::clone(&coordinator);
    let drain_handle = tokio::spawn(async move {
        coord_clone.wait_drain(Duration::from_millis(500)).await
    });

    // Inflight task finishes
    drop(slot1);

    let drain_res = drain_handle.await.expect("Join handle");
    assert!(drain_res.is_ok(), "Drain should complete cleanly");
    assert_eq!(coordinator.inflight_count(), 0);
}
