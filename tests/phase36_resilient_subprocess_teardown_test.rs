// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use aegis_gateway::backend::HermeticSubprocessBackend;
use aegis_gateway::core::backend::{BackendConfig, BackendTransport};
use aegis_gateway::core::session::{SessionFence, SessionLifecycleStatus};
use aegis_gateway::state::InMemorySessionFence;

#[tokio::test]
async fn test_phase36_time_bounded_subprocess_teardown() {
    let config = BackendConfig {
        name: "test_sleep_backend".to_string(),
        command: Some("sleep".to_string()),
        args: vec!["60".to_string()],
        env: HashMap::new(),
        url: None,
        timeout_secs: 5,
        enabled: true,
    };

    let backend = HermeticSubprocessBackend::new(config);
    backend.start().await.expect("Failed to start sleep backend");
    assert!(backend.is_healthy().await);

    let start = std::time::Instant::now();
    // Stop should time out after 1500ms and forcefully kill the sleep command within deadline
    backend.stop().await.expect("Failed to stop backend");
    let elapsed = start.elapsed();

    assert!(elapsed.as_millis() < 3000, "Teardown exceeded 3000ms deadline: {:?}", elapsed);
    assert!(!backend.is_healthy().await);
}

#[tokio::test]
async fn test_phase36_active_session_lease_protection() {
    let fence = InMemorySessionFence::new();
    let sess_id = "sess_active_streaming_901";

    fence.register_session(sess_id, "tenant_finance").await.unwrap();
    let lease_id = fence.acquire_lease(sess_id).await.unwrap();

    // While lease is active, attempt to terminate session (e.g. TTL reaper)
    fence.terminate_session(sess_id, "ttl_age_expired").await.unwrap();

    // Session status MUST be Terminating, NOT prematurely Tombstoned
    let status = fence.get_status(sess_id).await.unwrap();
    assert_eq!(status, SessionLifecycleStatus::Terminating);
    assert!(!fence.is_tombstoned(sess_id).await);

    // Release active lease after in-flight streaming finishes
    fence.release_lease(sess_id, lease_id).await.unwrap();

    // Now session transitions safely to Tombstoned
    let final_status = fence.get_status(sess_id).await.unwrap();
    assert_eq!(final_status, SessionLifecycleStatus::Tombstoned);
    assert!(fence.is_tombstoned(sess_id).await);
}
