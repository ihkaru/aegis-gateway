// SPDX-License-Identifier: MIT

use std::sync::Arc;

use aegis_gateway::control::{FileSignalConfigurationWatcher, HttpAdminControlPlane};
use aegis_gateway::core::control_plane::{
    ConfigurationWatcher, DynamicControlPlane, RuntimeBackendUpdate, RuntimePolicyUpdate,
};
use aegis_gateway::core::error::AegisError;

#[tokio::test]
async fn test_dynamic_backend_registration_and_removal_zero_downtime() {
    let cp = HttpAdminControlPlane::new();

    let initial_status = cp.get_status().await.expect("status check failed");
    assert_eq!(initial_status.config_version, 1);
    assert_eq!(initial_status.active_backends_count, 0);

    // 1. Dynamic backend registration
    let backend = RuntimeBackendUpdate {
        backend_name: "gdrive-connector".into(),
        transport: "sse".into(),
        endpoint_or_cmd: "https://gdrive.internal.svc:8484/sse".into(),
        enabled: true,
    };
    let v2 = cp.apply_backend_update(&backend).await.expect("apply backend failed");
    assert_eq!(v2, 2);

    let s2 = cp.get_status().await.expect("status check failed");
    assert_eq!(s2.active_backends_count, 1);

    // 2. Dynamic removal
    let removed = cp.remove_backend("gdrive-connector").await.expect("remove failed");
    assert!(removed);

    let s3 = cp.get_status().await.expect("status check failed");
    assert_eq!(s3.active_backends_count, 0);
    assert_eq!(s3.config_version, 3);
}

#[tokio::test]
async fn test_dynamic_policy_tier_hot_reload_and_validation() {
    let cp = HttpAdminControlPlane::new();

    // 1. Valid policy update (dev -> strict hot-reload)
    let pol_update = RuntimePolicyUpdate {
        tenant_id: "tenant-wealth-management".into(),
        policy_tier: "strict".into(),
        rate_limit_rps: 200,
        dlp_enabled: true,
    };
    let ver = cp.apply_policy_update(&pol_update).await.expect("policy apply failed");
    assert_eq!(ver, 2);

    let status = cp.get_status().await.expect("status failed");
    assert_eq!(status.active_policies_count, 1);

    // 2. Invalid policy tier rejected atomically
    let invalid_pol = RuntimePolicyUpdate {
        tenant_id: "tenant-wealth-management".into(),
        policy_tier: "unregulated_mode".into(),
        rate_limit_rps: 50,
        dlp_enabled: false,
    };
    let res = cp.apply_policy_update(&invalid_pol).await;
    match res {
        Err(AegisError::Validation(msg)) => {
            assert!(msg.contains("Invalid policy tier"));
        }
        other => panic!("Expected Validation error, got: {other:?}"),
    }

    // Verify state was not corrupted
    let status_after = cp.get_status().await.expect("status failed");
    assert_eq!(status_after.config_version, 2);
}

#[tokio::test]
async fn test_configuration_watcher_trigger() {
    let cp = Arc::new(HttpAdminControlPlane::new());
    let watcher = FileSignalConfigurationWatcher::new(cp.clone());

    // Register a backend to advance version
    let backend = RuntimeBackendUpdate {
        backend_name: "postgres-mcp".into(),
        transport: "stdio".into(),
        endpoint_or_cmd: "pg_mcp_server".into(),
        enabled: true,
    };
    cp.apply_backend_update(&backend).await.expect("apply failed");

    // Watcher triggers reload and observes latest version
    let ver = watcher.trigger_reload().await.expect("trigger reload failed");
    assert_eq!(ver, 2);
}
