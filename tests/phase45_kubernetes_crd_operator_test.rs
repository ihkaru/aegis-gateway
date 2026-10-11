// SPDX-License-Identifier: MIT
//! Phase 45: Declarative Kubernetes CRD Operator Test Suite
//! Validates GitOps CRD parsing, schema validation, zero-restart reconciliation, and checksum attestation.

use std::sync::Arc;

use propylon::control::{CrdOperatorEngine, HttpAdminControlPlane};
use propylon::core::control_plane::DynamicControlPlane;

const SAMPLE_K8S_MANIFEST: &str = r#"
apiVersion: propylon.enterprise.io/v1alpha1
kind: PropylonBackend
metadata:
  name: gdrive-connector
  namespace: propylon-production
spec:
  transport: sse
  endpoint_or_cmd: https://mcp-gdrive.internal.svc:8484/sse
  replicas: 3
  enabled: true
---
apiVersion: propylon.enterprise.io/v1alpha1
kind: PropylonPolicy
metadata:
  name: strict-banking-policy
  namespace: propylon-production
spec:
  tier: strict
  rateLimitRps: 100
  circuitBreakerThreshold: 5
  dlpEnabled: true
"#;

#[test]
fn test_phase45_parse_valid_k8s_crd_manifest() {
    let (backends, policies) = CrdOperatorEngine::parse_and_validate(SAMPLE_K8S_MANIFEST)
        .expect("manifest parsing must succeed");

    assert_eq!(backends.len(), 1);
    let b = &backends[0];
    assert_eq!(b.metadata.name, "gdrive-connector");
    assert_eq!(b.metadata.namespace, "propylon-production");
    assert_eq!(b.spec.transport, "sse");
    assert_eq!(b.spec.endpoint_or_cmd, "https://mcp-gdrive.internal.svc:8484/sse");
    assert_eq!(b.spec.replicas, 3);
    assert!(b.spec.enabled);

    assert_eq!(policies.len(), 1);
    let p = &policies[0];
    assert_eq!(p.metadata.name, "strict-banking-policy");
    assert_eq!(p.metadata.namespace, "propylon-production");
    assert_eq!(p.spec.tier, "strict");
    assert_eq!(p.spec.rate_limit_rps, 100);
    assert_eq!(p.spec.circuit_breaker_threshold, 5);
    assert!(p.spec.dlp_enabled);
}

#[test]
fn test_phase45_reject_invalid_api_version() {
    let invalid_manifest = r#"
apiVersion: propylon.enterprise.io/v99alpha99
kind: PropylonBackend
metadata:
  name: test-backend
spec:
  transport: sse
  endpoint_or_cmd: http://localhost:8080
"#;
    let err = CrdOperatorEngine::parse_and_validate(invalid_manifest)
        .expect_err("unsupported apiVersion must be rejected");
    assert!(err.to_string().contains("Unsupported apiVersion"));
}

#[test]
fn test_phase45_reject_invalid_transport() {
    let invalid_manifest = r#"
apiVersion: propylon.enterprise.io/v1alpha1
kind: PropylonBackend
metadata:
  name: test-backend
spec:
  transport: ftp_carrier_pigeon
  endpoint_or_cmd: ftp://localhost:8080
"#;
    let err = CrdOperatorEngine::parse_and_validate(invalid_manifest)
        .expect_err("invalid transport must be rejected");
    assert!(err.to_string().contains("Invalid transport"));
}

#[test]
fn test_phase45_reject_invalid_policy_tier() {
    let invalid_manifest = r#"
apiVersion: propylon.enterprise.io/v1alpha1
kind: PropylonPolicy
metadata:
  name: test-policy
spec:
  tier: yolo_unrestricted
  rateLimitRps: 100
"#;
    let err = CrdOperatorEngine::parse_and_validate(invalid_manifest)
        .expect_err("invalid policy tier must be rejected");
    assert!(err.to_string().contains("Invalid policy tier"));
}

#[tokio::test]
async fn test_phase45_live_reconciliation_zero_restart() {
    let cp = Arc::new(HttpAdminControlPlane::new());
    let operator = CrdOperatorEngine::new(cp.clone());

    let report = operator
        .reconcile(SAMPLE_K8S_MANIFEST)
        .await
        .expect("reconciliation must succeed");

    assert_eq!(report.applied_backends, vec!["gdrive-connector"]);
    assert_eq!(report.applied_policies, vec!["strict-banking-policy"]);
    assert_eq!(report.active_tier, Some("strict".to_string()));
    assert!(report.control_plane_version >= 2);
    assert_eq!(report.checksum_sha256.len(), 64);

    // Verify control plane reflects reconciled state
    let status = cp.get_status().await.expect("control plane status");
    assert_eq!(status.active_backends_count, 1);
    assert_eq!(status.active_policies_count, 1);
    assert!(status.config_version >= 2);
}
