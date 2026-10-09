// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

//! E2E Tier 6: Senior Enterprise Architect Chaos & Resilience Test Suite
//! Persona: Senior Enterprise Architect and SRE Chaos Engineer executing fault injection,
//! verifying Circuit Breaker tripping under cascading failure, subprocess crash handling,
//! and FinOps hard freeze under runaway agent loops.

use std::collections::HashMap;

use aegis_gateway::backend::HermeticSubprocessBackend;
use aegis_gateway::core::backend::{BackendConfig, BackendTransport};
use aegis_gateway::core::state::{BudgetManager, DistributedCircuitBreaker, QuotaEngine, TenantBudget};
use aegis_gateway::core::transport::{JsonRpcId, JsonRpcRequest};
use aegis_gateway::core::types::TenantId;
use aegis_gateway::state::{HardFreezeQuota, InMemoryStateBackend};

#[tokio::test]
async fn test_chaos_circuit_breaker_tripping_and_reset() {
    let state = InMemoryStateBackend::new();
    let upstream_service = "unreliable-upstream-api";

    // 1. Initial State: Circuit is Closed (Available)
    assert!(
        state.is_available(upstream_service).await.expect("check initial"),
        "Circuit breaker should initially be Closed"
    );

    // 2. Fault Injection: Inject 5 consecutive failures
    for _ in 0..5 {
        state.record_failure(upstream_service).await.expect("record failure");
    }

    // 3. Tripped State: Circuit is Open (Fast-Fail, calls blocked)
    assert!(
        !state.is_available(upstream_service).await.expect("check tripped"),
        "Circuit breaker must Trip to Open after threshold failures"
    );

    // 4. Recovery: Upstream recovers and succeeds
    state.record_success(upstream_service).await.expect("record recovery");

    // 5. Reset State: Circuit returns to Closed (Available)
    assert!(
        state.is_available(upstream_service).await.expect("check recovered"),
        "Circuit breaker must Reset to Closed upon successful probe"
    );
}

#[tokio::test]
async fn test_chaos_subprocess_crash_recovery_and_isolation() {
    // Subprocess exits immediately when it receives a crash command
    let backend = HermeticSubprocessBackend::new(BackendConfig {
        name: "crash-test-backend".to_string(),
        command: Some("python3".to_string()),
        args: vec![
            "-u".to_string(),
            "-c".to_string(),
            r#"
import sys, json
for line in sys.stdin:
    req = json.loads(line)
    if req.get("method") == "crash_now":
        sys.exit(1) # Intentional crash
    sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":req["id"],"result":{"ok":True}}) + "\n")
    sys.stdout.flush()
"#.to_string(),
        ],
        env: HashMap::new(),
        url: None,
        timeout_secs: 5,
        enabled: true,
    });

    backend.start().await.expect("spawn process");

    // Normal request works
    let req1 = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(1)),
        method: "normal_call".to_string(),
        params: None,
    };
    let resp1 = backend.send_request(&req1).await.expect("send normal");
    assert!(resp1.result.is_some());

    // Chaos request causes subprocess to exit
    let crash_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(2)),
        method: "crash_now".to_string(),
        params: None,
    };
    let crash_resp = backend.send_request(&crash_req).await.expect("send crash");
    // Gateway must handle the closed pipe gracefully with an internal error response without panicking
    assert!(
        crash_resp.error.is_some(),
        "Subprocess sudden crash must produce graceful JSON-RPC error response"
    );

    backend.stop().await.expect("stop backend");
}

#[tokio::test]
async fn test_chaos_finops_hard_freeze_runaway_loop_mitigation() {
    let quota = HardFreezeQuota::new();
    let tenant = TenantId::new("rogue-agent-tenant");

    // Allocate $100 budget
    quota
        .set_budget(TenantBudget {
            tenant_id: tenant.clone(),
            department: "AI Lab".to_string(),
            monthly_limit_usd: 100.0,
            current_spend_usd: 0.0,
            tokens_consumed: 0,
            hard_freeze_enabled: true,
        })
        .await
        .expect("set budget");

    // Simulate runaway loop: Agent rapidly drains tokens and budget
    quota.record_spend(&tenant, 40.0).await.expect("spend 1");
    assert!(quota.check_budget(&tenant).await.expect("check 1"));

    quota.record_spend(&tenant, 40.0).await.expect("spend 2"); // spend $80
    assert!(quota.check_budget(&tenant).await.expect("check 2"));

    quota.record_spend(&tenant, 25.0).await.expect("spend 3"); // spend $105 -> Exceeds $100

    // Hard freeze must trigger, halting the rogue autonomous loop
    let is_allowed = quota.check_budget(&tenant).await.expect("check frozen");
    assert!(
        !is_allowed,
        "FinOps Hard Freeze must block runaway autonomous loop when budget cap is exceeded"
    );
}
