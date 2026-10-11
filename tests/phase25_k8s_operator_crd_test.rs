// SPDX-License-Identifier: MIT

use serde_json::json;

use propylon::core::k8s::{
    PropylonBackendCrd, PropylonCrdReconciler, PropylonPolicyCrd, PropylonTenantCrd,
};
use propylon::k8s::DeclarativeCrdController;

#[tokio::test]
async fn test_declarative_crd_reconciliation() {
    let controller = DeclarativeCrdController::new("ghcr.io/propylon/proxy:latest");

    // 1. Reconcile PropylonBackend
    let backend = PropylonBackendCrd {
        name: "python-datascience-mcp".into(),
        namespace: "propylon-workloads".into(),
        transport: "stdio".into(),
        endpoint_or_cmd: "python3 /app/mcp_server.py".into(),
        replicas: 2,
        enabled: true,
    };
    let outcome = controller.reconcile_backend(&backend).await.expect("reconcile backend failed");
    assert_eq!(outcome.status, "Applied");

    // 2. Reconcile PropylonPolicy
    let policy = PropylonPolicyCrd {
        name: "strict-financial-governance".into(),
        namespace: "propylon-workloads".into(),
        tier: "strict".into(),
        rate_limit_rps: 50,
        circuit_breaker_threshold: 5,
        dlp_enabled: true,
    };
    let outcome_pol = controller.reconcile_policy(&policy).await.expect("reconcile policy failed");
    assert_eq!(outcome_pol.status, "Applied");

    // 3. Reconcile PropylonTenant
    let tenant = PropylonTenantCrd {
        tenant_id: "analytics-bi-team".into(),
        namespace: "propylon-workloads".into(),
        monthly_token_budget: 5_000_000,
        cost_center: "CC-9012".into(),
    };
    let outcome_t = controller.reconcile_tenant(&tenant).await.expect("reconcile tenant failed");
    assert_eq!(outcome_t.status, "Applied");
}

#[tokio::test]
async fn test_admission_webhook_validation_refuses_invalid_specs() {
    let controller = DeclarativeCrdController::new("ghcr.io/propylon/proxy:latest");

    // Invalid transport
    let invalid_backend_json = json!({
        "spec": {
            "transport": "ftp",
            "endpoint_or_cmd": "ftp://files.internal"
        }
    });
    let decision = controller
        .validate_admission("PropylonBackend", &invalid_backend_json)
        .await
        .expect("validation failed");
    assert!(!decision.allowed);
    assert_eq!(decision.status_code, 422);

    // Valid backend
    let valid_backend_json = json!({
        "spec": {
            "transport": "sse",
            "endpoint_or_cmd": "https://mcp.internal.svc:8484/sse"
        }
    });
    let decision_valid = controller
        .validate_admission("PropylonBackend", &valid_backend_json)
        .await
        .expect("validation failed");
    assert!(decision_valid.allowed);
    assert_eq!(decision_valid.status_code, 200);
}

#[tokio::test]
async fn test_mutating_webhook_injects_sidecar_proxy() {
    let controller = DeclarativeCrdController::new("ghcr.io/propylon/proxy:v1.0.0");

    let initial_pod = json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "claude-code-agent-pod" },
        "spec": {
            "containers": [
                {
                    "name": "agent-worker",
                    "image": "anthropic/claude-agent:latest"
                }
            ]
        }
    });

    // 1. Mutate pod spec
    let mutated = controller.mutate_pod_spec(&initial_pod).await.expect("mutation failed");
    let containers = mutated["spec"]["containers"].as_array().expect("containers must be array");
    assert_eq!(containers.len(), 2);

    let sidecar = containers.iter().find(|c| c["name"] == "propylon-sidecar-proxy").expect("sidecar missing");
    assert_eq!(sidecar["image"], "ghcr.io/propylon/proxy:v1.0.0");

    // 2. Ensure idempotent mutation (running again does not duplicate)
    let re_mutated = controller.mutate_pod_spec(&mutated).await.expect("second mutation failed");
    let re_containers = re_mutated["spec"]["containers"].as_array().expect("re-containers array");
    assert_eq!(re_containers.len(), 2);
}
