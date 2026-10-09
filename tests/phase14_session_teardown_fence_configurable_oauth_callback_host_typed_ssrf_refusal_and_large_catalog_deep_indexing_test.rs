//! Phase 14 Integration Test Suite: Session Teardown Fence, Configurable OAuth Callback Host, Typed SSRF Refusal and Large Catalog Deep Indexing
//! Target Gaps: MikkoParkkola/mcp-gateway#2568, MikkoParkkola/mcp-gateway#2578, MikkoParkkola/mcp-gateway#2508, MikkoParkkola/mcp-gateway#3034

#![deny(unsafe_code)]

use std::collections::HashMap;
use serde_json::json;

use aegis_gateway::core::session::{SessionFence, SessionLifecycleStatus};
use aegis_gateway::core::types::{DisclosureTier, ToolDefinition};
use aegis_gateway::discovery::{
    BackendCatalogMetadata, BackendIndexStatus, CatalogSearchIndex,
};
use aegis_gateway::policy::{
    CallbackServerConfig, OAuthCallbackResolver, SecurityPosture, SsrfRedirectValidator,
};
use aegis_gateway::state::session_fence::InMemorySessionFence;

#[tokio::test]
async fn test_mikko_issue_2568_in_flight_session_teardown_fence_and_anti_resurrection() {
    let fence = InMemorySessionFence::new();

    // 1. Register active session
    fence
        .register_session("sess-mikko-2568", "tenant-alpha")
        .await
        .expect("Registration should succeed");

    assert_eq!(
        fence.get_status("sess-mikko-2568").await.unwrap(),
        SessionLifecycleStatus::Active
    );

    // 2. Worker 1 acquires lease for in-flight tool call
    let lease = fence
        .acquire_lease("sess-mikko-2568")
        .await
        .expect("Lease acquisition must succeed for active session");
    assert_eq!(fence.active_leases("sess-mikko-2568").await, 1);

    // Initial state write is allowed while active
    fence
        .write_session_state("sess-mikko-2568", "start_ts", json!(1000))
        .await
        .expect("Write must succeed for active session");

    // 3. Admin / Reaper terminates session while call is in flight
    fence
        .terminate_session("sess-mikko-2568", "Session TTL expired during long execution")
        .await
        .expect("Termination should succeed");

    // Status is Terminating because lease is active
    assert_eq!(
        fence.get_status("sess-mikko-2568").await.unwrap(),
        SessionLifecycleStatus::Terminating
    );

    // 4. New lease acquisition MUST be blocked
    let new_lease = fence.acquire_lease("sess-mikko-2568").await;
    assert!(new_lease.is_err(), "New leases must be rejected during teardown");

    // 5. In-flight call completes and tries to write state (e.g. cost/tokens under dead id)
    // Issue #2568: write must be refused so dead session is not resurrected or populated with orphaned state
    let dead_write = fence
        .write_session_state("sess-mikko-2568", "cost_usd", json!(0.045))
        .await;
    assert!(
        dead_write.is_err(),
        "Writing state to terminated session must be refused"
    );

    // 6. In-flight worker finishes and releases lease
    fence
        .release_lease("sess-mikko-2568", lease)
        .await
        .expect("Lease release should succeed");

    assert_eq!(fence.active_leases("sess-mikko-2568").await, 0);
    assert_eq!(
        fence.get_status("sess-mikko-2568").await.unwrap(),
        SessionLifecycleStatus::Tombstoned
    );

    // 7. Verify session cannot be resurrected
    let resurrect = fence.register_session("sess-mikko-2568", "tenant-alpha").await;
    assert!(
        resurrect.is_err(),
        "Tombstoned session ID must not be resurrected"
    );
}

#[test]
fn test_mikko_issue_2578_configurable_oauth_callback_host_and_reverse_proxy() {
    // 1. Default localhost behavior
    let default_resolver = OAuthCallbackResolver::new(CallbackServerConfig::default());
    let default_url = default_resolver.resolve_redirect_uri(None);
    assert_eq!(default_url, "http://localhost:8080/oauth/callback");

    // 2. Issue #2578: operator specifies 127.0.0.1 to avoid broken IPv6 ::1 dual-bind
    let ip_resolver = OAuthCallbackResolver::with_host_and_port("127.0.0.1", 8484);
    let ip_url = ip_resolver.resolve_redirect_uri(None);
    assert_eq!(
        ip_url, "http://127.0.0.1:8484/oauth/callback",
        "Configured IP must be preserved in redirect URI"
    );
    assert!(!ip_url.contains("localhost"), "Must not falsely default to localhost");

    // 3. IPv6 literal bracketing
    let ipv6_resolver = OAuthCallbackResolver::with_host_and_port("::1", 9090);
    let ipv6_url = ipv6_resolver.resolve_redirect_uri(None);
    assert_eq!(ipv6_url, "http://[::1]:9090/oauth/callback");

    // 4. Reverse proxy headers when trust_proxy_headers is enabled
    let proxy_config = CallbackServerConfig {
        callback_host: "127.0.0.1".to_string(),
        callback_port: 8080,
        callback_path: "/auth/mcp/callback".to_string(),
        scheme: "http".to_string(),
        trust_proxy_headers: true,
    };
    let proxy_resolver = OAuthCallbackResolver::new(proxy_config);

    let mut proxy_headers = HashMap::new();
    proxy_headers.insert("x-forwarded-host".to_string(), "mcp.gateway.enterprise.com".to_string());
    proxy_headers.insert("x-forwarded-proto".to_string(), "https".to_string());

    let proxy_url = proxy_resolver.resolve_redirect_uri(Some(&proxy_headers));
    assert_eq!(
        proxy_url,
        "https://mcp.gateway.enterprise.com/auth/mcp/callback"
    );
}

#[test]
fn test_mikko_issue_2508_typed_ssrf_redirect_refusal_and_json_rpc_code() {
    let validator = SsrfRedirectValidator::new(SecurityPosture::Hardened);

    // 1. AWS Cloud Metadata Hop
    let aws_res = validator.validate_redirect_target("http://169.254.169.254/latest/meta-data/");
    assert!(aws_res.is_err());
    let err = aws_res.unwrap_err();
    assert_eq!(err.to_rpc_code(), -32600, "Must return JSON-RPC -32600");
    assert!(
        err.to_string().starts_with("SSRF blocked:"),
        "Error message must start with SSRF blocked"
    );

    // 2. Private Subnet Hop (RFC 1918)
    let private_res = validator.validate_redirect_target("http://10.244.0.15:8080/oauth/token");
    assert!(private_res.is_err());
    let priv_err = private_res.unwrap_err();
    assert_eq!(priv_err.to_rpc_code(), -32600);
    assert!(priv_err.to_string().contains("private/metadata destination"));

    // 3. Prohibited Scheme Hop
    let file_res = validator.validate_redirect_target("file:///etc/passwd");
    assert!(file_res.is_err());
    assert_eq!(file_res.unwrap_err().to_rpc_code(), -32600);

    // 4. Valid Public Identity Provider
    let public_res = validator
        .validate_redirect_target("https://login.microsoftonline.com/v2.0/.well-known/openid-configuration");
    assert!(public_res.is_ok(), "Public IdP destination must be allowed");
}

#[test]
fn test_mikko_issue_3034_catalog_search_deep_indexing_and_unindexed_visibility() {
    let mut catalog = CatalogSearchIndex::new();

    // 1. Register backend with dynamic toolset (44 dbhub tools)
    catalog.register_backend(BackendCatalogMetadata {
        name: "dbhub".to_string(),
        description: "Bytebase DBHub fleet database and unified query engine".to_string(),
        tags: vec!["database".to_string(), "sql".to_string()],
        status: BackendIndexStatus::Indexed,
    });

    let mut dbhub_tools = Vec::new();
    for i in 1..=44 {
        let name = if i == 1 {
            "execute_sql_homarr".to_string()
        } else {
            format!("execute_sql_cluster_{}", i)
        };
        dbhub_tools.push(ToolDefinition {
            name,
            server: "dbhub".to_string(),
            description: format!("Execute database query against SQL cluster #{}", i),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "SQL statement" }
                },
                "required": ["query"]
            }),
            required_params: vec!["query".to_string()],
            when_to_use: "When running relational SQL operations".to_string(),
            tags: vec!["db".to_string(), "postgres".to_string()],
        });
    }

    catalog.index_tools_for_backend("dbhub", dbhub_tools);
    assert_eq!(catalog.backend_tool_count("dbhub"), 44);

    // 2. Register an unindexed / initializing backend
    catalog.mark_backend_initializing("k8s_operator", "Kubernetes cluster orchestrator");

    // 3. Query specific dynamic tool: 'homarr'
    let homarr_res = catalog.search("homarr", DisclosureTier::L0, 10);
    assert_eq!(homarr_res.total_matches, 1);
    assert_eq!(homarr_res.tools[0].name, "execute_sql_homarr");

    // 4. Query matching backend description only: 'Bytebase DBHub fleet'
    // In issue #3034, searching for backend description returned 0 dbhub tools.
    // In Aegis, all 44 tools are returned because parent metadata is deeply indexed!
    let fleet_res = catalog.search("Bytebase DBHub fleet", DisclosureTier::L0, 50);
    assert_eq!(
        fleet_res.total_matches, 44,
        "Deep backend description indexing must surface all 44 tools"
    );

    // 5. Query with 0 matches reports unindexed backends so caller does not assume failure
    let zero_res = catalog.search("quantum_crypto_xyz", DisclosureTier::L0, 10);
    assert_eq!(zero_res.total_matches, 0);
    assert!(
        zero_res.unindexed_backends.contains(&"k8s_operator".to_string()),
        "Search miss must report initializing/unindexed backends for diagnostic clarity"
    );
}
