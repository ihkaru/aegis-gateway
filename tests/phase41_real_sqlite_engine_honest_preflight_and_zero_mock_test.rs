// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

use serde_json::json;
use propylon::backend::LocalPropylonBackend;
use propylon::control::web_ui::EmbeddedAdminServer;
use propylon::core::backend::BackendTransport;
use propylon::core::transport::{JsonRpcId, JsonRpcRequest};

#[tokio::test]
async fn test_phase41_local_sqlite_live_query_and_mutation() {
    let backend = LocalPropylonBackend::new();
    assert!(backend.is_healthy().await);

    // 1. Initialize audit_ledger or customer_accounts via live query
    let init_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(1)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "propylon_sqlite_query",
            "arguments": {
                "query": "CREATE TABLE IF NOT EXISTS test_vault (id INTEGER PRIMARY KEY AUTOINCREMENT, item_key TEXT UNIQUE, val REAL);"
            }
        })),
    };

    let init_resp = backend.send_request(&init_req).await.unwrap();
    assert!(init_resp.error.is_none());
    let init_data = init_resp.result.unwrap()["data"].clone();
    assert_eq!(init_data["status"], "SUCCESS");

    // 2. Perform live mutation (INSERT)
    let insert_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(2)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "propylon_sqlite_query",
            "arguments": {
                "query": "INSERT OR REPLACE INTO test_vault (item_key, val) VALUES ('SOC2_ASSET_DELTA', 45000.75);"
            }
        })),
    };

    let insert_resp = backend.send_request(&insert_req).await.unwrap();
    let insert_data = insert_resp.result.unwrap()["data"].clone();
    assert_eq!(insert_data["status"], "SUCCESS");
    assert_eq!(insert_data["type"], "MUTATION");

    // 3. Perform live retrieval (SELECT) and verify persistent row
    let select_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(3)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "propylon_sqlite_query",
            "arguments": {
                "query": "SELECT item_key, val FROM test_vault WHERE item_key = 'SOC2_ASSET_DELTA';"
            }
        })),
    };

    let select_resp = backend.send_request(&select_req).await.unwrap();
    let select_data = select_resp.result.unwrap()["data"].clone();
    assert_eq!(select_data["status"], "SUCCESS");
    assert_eq!(select_data["type"], "SELECT");
    assert_eq!(select_data["row_count"], 1);
    let rows = select_data["rows"].as_array().unwrap();
    assert_eq!(rows[0]["item_key"], "SOC2_ASSET_DELTA");
    assert_eq!(rows[0]["val"], 45000.75);
}

#[tokio::test]
async fn test_phase41_honest_unattached_tool_preflight_handling() {
    let backend = LocalPropylonBackend::new();

    // Call unattached catalog tool (destructive_drop_table or query_postgresql_ledger)
    let unattached_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(10)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "query_postgresql_ledger",
            "arguments": { "query": "SELECT * FROM external_db;" }
        })),
    };

    let resp = backend.send_request(&unattached_req).await.unwrap();
    let data = resp.result.unwrap()["data"].clone();

    // Must be marked unattached and PREFLIGHT_GUARDRAIL_ONLY, zero fake success
    assert_eq!(data["status"], "unattached");
    assert_eq!(data["execution_mode"], "PREFLIGHT_GUARDRAIL_ONLY");
    assert_eq!(data["upstream_status"], "NOT_CONNECTED");
    assert!(data["message"].as_str().unwrap().contains("No live upstream"));
}

#[tokio::test]
async fn test_phase41_embedded_admin_ui_catalog_and_playground_dispatch() {
    let server = EmbeddedAdminServer::new(8485, true);

    // 1. GET /api/v1/tools returns live tools alongside template catalog
    let (tools_status, tools_body, _) = server.handle_request("GET", "/api/v1/tools").await;
    assert_eq!(tools_status, 200);
    let tools: Vec<serde_json::Value> = serde_json::from_slice(&tools_body).unwrap();
    assert!(tools.len() >= 3);

    let has_sqlite = tools.iter().any(|t| t["name"] == "propylon_sqlite_query" && t["is_live"] == true);
    let has_health = tools.iter().any(|t| t["name"] == "propylon_system_health" && t["is_live"] == true);
    let has_postgres_template = tools.iter().any(|t| t["name"] == "query_postgresql_ledger" && t["is_live"] == false);

    assert!(has_sqlite, "Expected propylon_sqlite_query to be marked live");
    assert!(has_health, "Expected propylon_system_health to be marked live");
    assert!(has_postgres_template, "Expected query_postgresql_ledger template to be unattached");

    // 2. Dispatch real SQLite tool from UI API
    let sqlite_dispatch = json!({
        "tool": "propylon_sqlite_query",
        "arguments": { "query": "SELECT 1 AS live_check;" }
    });
    let (exec_status, exec_body, _) = server
        .handle_request_with_body("POST", "/api/v1/tools/call", &serde_json::to_vec(&sqlite_dispatch).unwrap())
        .await;
    assert_eq!(exec_status, 200);
    let exec_json: serde_json::Value = serde_json::from_slice(&exec_body).unwrap();
    assert_eq!(exec_json["outcome"], "DISPATCHED");
    assert_eq!(exec_json["result"]["execution_mode"], "LIVE_EXECUTED");
    assert_eq!(exec_json["raw"]["data"]["status"], "SUCCESS");

    // 3. Dispatch unattached tool from UI API (Honest preflight)
    let unattached_dispatch = json!({
        "tool": "query_postgresql_ledger",
        "arguments": { "query": "SELECT * FROM remote_orders;" }
    });
    let (unatt_status, unatt_body, _) = server
        .handle_request_with_body("POST", "/api/v1/tools/call", &serde_json::to_vec(&unattached_dispatch).unwrap())
        .await;
    assert_eq!(unatt_status, 200);
    let unatt_json: serde_json::Value = serde_json::from_slice(&unatt_body).unwrap();
    assert_eq!(unatt_json["outcome"], "DISPATCHED");
    assert_eq!(unatt_json["result"]["execution_mode"], "PREFLIGHT_GUARDRAIL_ONLY");
    assert_eq!(unatt_json["raw"]["data"]["status"], "unattached");

    // 4. Dispatch sqlite_query (live tool) from UI API -> Must be LIVE_EXECUTED
    let alias_dispatch = json!({
        "tool": "sqlite_query",
        "arguments": { "query": "SELECT 42 AS num;" }
    });
    let (alias_status, alias_body, _) = server
        .handle_request_with_body("POST", "/api/v1/tools/call", &serde_json::to_vec(&alias_dispatch).unwrap())
        .await;
    assert_eq!(alias_status, 200);
    let alias_json: serde_json::Value = serde_json::from_slice(&alias_body).unwrap();
    assert_eq!(alias_json["outcome"], "DISPATCHED");
    assert_eq!(alias_json["result"]["execution_mode"], "LIVE_EXECUTED");
    assert_eq!(alias_json["raw"]["data"]["status"], "SUCCESS");
}

#[tokio::test]
async fn test_phase41_interface_first_sql_engine_contract() {
    use std::sync::Arc;
    use propylon::core::backend::BackendRegistry;
    use propylon::core::database::{SqlEngine, SqlQueryResult};
    use propylon::backend::LocalSqliteEngine;

    // 1. Verify SqlEngine abstraction trait with dynamic dispatch (DIP & LSP)
    let engine: Arc<dyn SqlEngine> = Arc::new(LocalSqliteEngine::new("data/contract_test.db"));
    assert!(engine.is_healthy().await);
    assert_eq!(engine.engine_name(), "sqlite-hermetic-local");

    // 2. Execute mutation through abstraction
    let res = engine.execute("CREATE TABLE IF NOT EXISTS contract_nodes (id INTEGER PRIMARY KEY, label TEXT);").await.unwrap();
    match res {
        SqlQueryResult::Mutation { .. } => {}
        _ => panic!("Expected mutation result"),
    }

    // 3. Inject into LocalPropylonBackend via Dependency Injection (DI Pattern)
    let di_backend = LocalPropylonBackend::with_sql_engine(engine);
    assert!(di_backend.is_healthy().await);

    // 4. Test BackendRegistry find_backend_for_tool contract
    let registry = propylon::backend::SubprocessBackendRegistry::with_default_local_engine();
    let found = registry.find_backend_for_tool("propylon_sqlite_query").await.unwrap();
    assert!(found.is_some(), "Expected to resolve backend for propylon_sqlite_query");

    let not_found = registry.find_backend_for_tool("non_existent_tool").await.unwrap();
    assert!(not_found.is_none(), "Expected None for non-existent tool");
}

