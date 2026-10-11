// SPDX-License-Identifier: MIT

use serde_json::json;
use std::sync::Arc;

use aegis_gateway::backend::SubprocessBackendRegistry;
use aegis_gateway::control::ConfigApiDispatcher;
use aegis_gateway::core::backend::BackendRegistry;
use aegis_gateway::discovery::default_enterprise_catalog;

#[test]
fn test_catalog_purification_zero_synthetic_workers() {
    let catalog = default_enterprise_catalog();

    // Verify no tool has synthetic dummy or worker task names
    for tool in &catalog {
        assert!(
            !tool.name.contains("worker_task"),
            "Synthetic worker task detected in purified catalog: {}",
            tool.name
        );
        assert!(
            !tool.name.starts_with("dummy_"),
            "Synthetic dummy tool detected: {}",
            tool.name
        );
        assert!(
            !tool.name.starts_with("fake_"),
            "Synthetic fake tool detected: {}",
            tool.name
        );
    }

    // Verify genuine enterprise integration tools exist
    let tool_names: Vec<String> = catalog.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.contains(&"query_postgresql_ledger".to_string()));
    assert!(tool_names.contains(&"upload_to_storage".to_string()));
    assert!(tool_names.contains(&"stripe_create_customer".to_string()));
    assert!(tool_names.contains(&"dispatch_slack_alert".to_string()));
    assert!(tool_names.contains(&"transcode_media_video".to_string()));

    // Verify catalog size is clean and focused (18 genuine tools)
    assert_eq!(catalog.len(), 18, "Catalog must contain exactly 18 enterprise tools");
}

#[tokio::test]
async fn test_config_api_raw_and_template() {
    // 1. Raw config fetch
    let (status, body, mime) = ConfigApiDispatcher::dispatch("GET", "/api/v1/config/raw", &[], None).await;
    assert_eq!(status, 200);
    assert_eq!(mime, "application/json");

    let val: serde_json::Value = serde_json::from_slice(&body).expect("Must parse valid JSON");
    assert_eq!(val["status"], "LOADED");
    assert!(val["content"].is_string());
    assert!(val["path"].as_str().unwrap().contains("aegis"));

    // 2. Default starter template fetch
    let (status_t, body_t, mime_t) = ConfigApiDispatcher::dispatch("GET", "/api/v1/config/template", &[], None).await;
    assert_eq!(status_t, 200);
    assert_eq!(mime_t, "application/json");

    let val_t: serde_json::Value = serde_json::from_slice(&body_t).expect("Must parse valid JSON");
    assert_eq!(val_t["format"], "yaml");
    let content_t = val_t["content"].as_str().unwrap();
    assert!(content_t.contains("mcp_servers:"));
    assert!(content_t.contains("default_tenant:"));
}

#[tokio::test]
async fn test_config_api_syntax_validation() {
    // 1. Valid topology YAML
    let valid_yaml = r#"
mcp_servers:
  sqlite_ledger:
    name: sqlite_ledger
    command: python3
    args:
    - scripts/sqlite_mcp_server.py
    timeout_secs: 30
    enabled: true
default_tenant: enterprise-corp
"#;
    let req_body = json!({ "content": valid_yaml, "format": "yaml" }).to_string().into_bytes();
    let (status, body, _) = ConfigApiDispatcher::dispatch("POST", "/api/v1/config/validate", &req_body, None).await;
    assert_eq!(status, 200);

    let res: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
    assert_eq!(res["valid"], true);
    assert_eq!(res["backends_count"], 1);
    assert_eq!(res["default_tenant"], "enterprise-corp");

    // 2. Invalid YAML syntax (broken indent/schema)
    let broken_yaml = "mcp_servers:\n  broken_backend: [unclosed list";
    let req_body_bad = json!({ "content": broken_yaml, "format": "yaml" }).to_string().into_bytes();
    let (status_bad, body_bad, _) = ConfigApiDispatcher::dispatch("POST", "/api/v1/config/validate", &req_body_bad, None).await;
    assert_eq!(status_bad, 422);

    let res_bad: serde_json::Value = serde_json::from_slice(&body_bad).expect("JSON response");
    assert_eq!(res_bad["valid"], false);
    assert!(res_bad["error"].is_string());

    // 3. Semantic validation error: backend without command or url
    let incomplete_yaml = r#"
mcp_servers:
  no_transport_backend:
    timeout_secs: 15
    enabled: true
"#;
    let req_body_inc = json!({ "content": incomplete_yaml, "format": "yaml" }).to_string().into_bytes();
    let (status_inc, body_inc, _) = ConfigApiDispatcher::dispatch("POST", "/api/v1/config/validate", &req_body_inc, None).await;
    assert_eq!(status_inc, 422);

    let res_inc: serde_json::Value = serde_json::from_slice(&body_inc).expect("JSON response");
    assert_eq!(res_inc["valid"], false);
    assert!(res_inc["error"].as_str().unwrap().contains("must specify either 'command' or 'url'"));
}

#[tokio::test]
async fn test_config_api_apply_atomic_hot_reload() {
    let registry = Arc::new(SubprocessBackendRegistry::with_default_local_engine());
    let reg_dyn: Arc<dyn BackendRegistry> = registry.clone();

    let valid_yaml = r#"
mcp_servers:
  remote_analytics:
    name: remote_analytics
    url: https://analytics.corp.internal/mcp
    timeout_secs: 20
    enabled: true
default_tenant: finops-unit
"#;
    let req_body = json!({ "content": valid_yaml, "format": "yaml" }).to_string().into_bytes();
    let (status, body, _) = ConfigApiDispatcher::dispatch("POST", "/api/v1/config/apply", &req_body, Some(&reg_dyn)).await;
    assert_eq!(status, 200);

    let res: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
    assert_eq!(res["status"], "APPLIED");
    assert_eq!(res["backends_count"], 1);

    // Verify hot-reloaded backend is live in registry without service restart
    let active_backends = registry.list_backends().await.expect("List backends");
    assert!(
        active_backends.contains(&"remote_analytics".to_string()),
        "Registry must dynamically contain hot-reloaded backend"
    );
}
