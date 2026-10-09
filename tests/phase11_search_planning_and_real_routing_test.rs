// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use aegis_gateway::backend::SubprocessBackendRegistry;
use aegis_gateway::core::backend::{BackendRegistry, BackendTransport};
use aegis_gateway::core::error::AegisResult;
use aegis_gateway::core::transport::{JsonRpcRequest, JsonRpcResponse, WireProtocolHandler};
use aegis_gateway::core::types::ToolDefinition;
use aegis_gateway::discovery::{ExecutionPlan, ExecutionPlanner, PlanRisk, PlanStep};
use aegis_gateway::policy::AbacPolicyEngine;
use aegis_gateway::transport::McpProtocolHandler;
use aegis_gateway::AegisGateway;

/// Concrete test transport to verify real asynchronous request forwarding
struct RecordingBackendTransport {
    call_count: Arc<AtomicUsize>,
}

#[async_trait]
impl BackendTransport for RecordingBackendTransport {
    async fn start(&self) -> AegisResult<()> {
        Ok(())
    }

    async fn send_request(&self, request: &JsonRpcRequest) -> AegisResult<JsonRpcResponse> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let id = request.id.clone().unwrap_or_default();
        let name = request
            .params
            .as_ref()
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        Ok(JsonRpcResponse::success(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": format!("REAL_BACKEND_EXECUTION_RESULT_FOR_{name}")
                }]
            }),
        ))
    }

    async fn is_healthy(&self) -> bool {
        true
    }

    async fn stop(&self) -> AegisResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_real_backend_execution_routing_without_mocks() {
    let gateway = Arc::new(AegisGateway::default());
    let registry = Arc::new(SubprocessBackendRegistry::new());
    let call_count = Arc::new(AtomicUsize::new(0));

    let transport: Arc<dyn BackendTransport> = Arc::new(RecordingBackendTransport {
        call_count: Arc::clone(&call_count),
    });
    registry.register("postgres", transport).await.unwrap();

    let handler = Arc::new(McpProtocolHandler::with_registry(
        Arc::clone(&gateway),
        Some(Arc::clone(&registry) as Arc<dyn BackendRegistry>),
    ));

    handler
        .register_tools(vec![ToolDefinition {
            name: "query_database".to_string(),
            server: "postgres".to_string(),
            description: "Execute SQL query".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec!["sql".to_string()],
            when_to_use: "Query database".to_string(),
            tags: vec!["postgres".to_string()],
        }])
        .await;

    // Call tool through wire protocol
    let req = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "tools/call",
        "params": {
            "name": "query_database",
            "arguments": { "sql": "SELECT 1" }
        }
    });

    let resp_str = handler
        .handle_message(&req.to_string())
        .await
        .unwrap()
        .unwrap();
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();

    // Verify the real backend was called and returned actual text!
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "REAL_BACKEND_EXECUTION_RESULT_FOR_query_database");
    assert!(!text.contains("status"), "Must not return dummy status mock");
}

#[tokio::test]
async fn test_gateway_search_tools_progressive_discovery() {
    let gateway = Arc::new(AegisGateway::default());
    let handler = Arc::new(McpProtocolHandler::new(Arc::clone(&gateway)));

    handler
        .register_tools(vec![
            ToolDefinition {
                name: "git_commit".to_string(),
                server: "git".to_string(),
                description: "Record changes to the repository".to_string(),
                input_schema: json!({ "type": "object" }),
                required_params: vec!["message".to_string()],
                when_to_use: "Git commit".to_string(),
                tags: vec!["vcs".to_string()],
            },
            ToolDefinition {
                name: "find_users".to_string(),
                server: "postgres".to_string(),
                description: "Lookup users in database".to_string(),
                input_schema: json!({ "type": "object" }),
                required_params: vec!["query".to_string()],
                when_to_use: "User search".to_string(),
                tags: vec!["postgres".to_string()],
            },
        ])
        .await;

    // Search specifically for "postgres"
    let req = json!({
        "jsonrpc": "2.0",
        "id": 101,
        "method": "gateway_search_tools",
        "params": { "query": "postgres" }
    });

    let resp_str = handler
        .handle_message(&req.to_string())
        .await
        .unwrap()
        .unwrap();
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty());
    assert_eq!(tools[0]["name"], "find_users");
    assert_eq!(tools[0]["server"], "postgres");
}

#[tokio::test]
async fn test_execution_planner_dag_validation_and_risk_scoring() {
    let planner = ExecutionPlanner::new();
    let policy = AbacPolicyEngine::new();
    let caller = aegis_gateway::core::types::CallerContext {
        tenant_id: aegis_gateway::core::types::TenantId::new("engineering"),
        subject: "agent-1".to_string(),
        roles: vec!["developer".to_string()],
        department: Some("Engineering".to_string()),
        client_ip: None,
        session_id: "test-session".to_string(),
    };

    let catalog = vec![
        ToolDefinition {
            name: "read_schema".to_string(),
            server: "db".to_string(),
            description: "Read database schema".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec![],
            when_to_use: "Inspect schema".to_string(),
            tags: vec![],
        },
        ToolDefinition {
            name: "execute_sql".to_string(),
            server: "db".to_string(),
            description: "Execute SQL statement".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec!["query".to_string()],
            when_to_use: "Run SQL".to_string(),
            tags: vec![],
        },
    ];

    // 1. Valid linear DAG
    let valid_plan = ExecutionPlan {
        plan_id: "plan-valid".to_string(),
        title: "Migrate Schema".to_string(),
        steps: vec![
            PlanStep {
                id: "step-1".to_string(),
                tool: "read_schema".to_string(),
                server: Some("db".to_string()),
                arguments: json!({}),
                depends_on: vec![],
                description: "Inspect tables".to_string(),
            },
            PlanStep {
                id: "step-2".to_string(),
                tool: "execute_sql".to_string(),
                server: Some("db".to_string()),
                arguments: json!({ "query": "SELECT count(*) FROM users;" }),
                depends_on: vec!["step-1".to_string()],
                description: "Verify row count".to_string(),
            },
        ],
    };

    let res_valid = planner
        .validate_plan(&valid_plan, &catalog, &policy, &caller)
        .await;
    assert!(res_valid.valid);
    assert_eq!(res_valid.overall_risk, PlanRisk::Low);
    assert!(!res_valid.human_approval_required);

    // 2. Cyclic DAG: step-1 -> step-2 -> step-1
    let cyclic_plan = ExecutionPlan {
        plan_id: "plan-cycle".to_string(),
        title: "Circular Steps".to_string(),
        steps: vec![
            PlanStep {
                id: "step-1".to_string(),
                tool: "read_schema".to_string(),
                server: Some("db".to_string()),
                arguments: json!({}),
                depends_on: vec!["step-2".to_string()],
                description: "Cycle 1".to_string(),
            },
            PlanStep {
                id: "step-2".to_string(),
                tool: "execute_sql".to_string(),
                server: Some("db".to_string()),
                arguments: json!({ "query": "SELECT 1" }),
                depends_on: vec!["step-1".to_string()],
                description: "Cycle 2".to_string(),
            },
        ],
    };

    let res_cyclic = planner
        .validate_plan(&cyclic_plan, &catalog, &policy, &caller)
        .await;
    assert!(!res_cyclic.valid);
    assert!(res_cyclic
        .errors
        .iter()
        .any(|e| e.contains("Circular dependency")));

    // 3. High Risk Destructive Command requiring human approval
    let destructive_plan = ExecutionPlan {
        plan_id: "plan-destructive".to_string(),
        title: "Drop Production Table".to_string(),
        steps: vec![PlanStep {
            id: "step-1".to_string(),
            tool: "execute_sql".to_string(),
            server: Some("db".to_string()),
            arguments: json!({ "query": "DROP TABLE critical_customers;" }),
            depends_on: vec![],
            description: "Dangerous drop".to_string(),
        }],
    };

    let res_dest = planner
        .validate_plan(&destructive_plan, &catalog, &policy, &caller)
        .await;
    assert_eq!(res_dest.overall_risk, PlanRisk::Critical);
    assert!(res_dest.human_approval_required);
}

#[tokio::test]
async fn test_gateway_plan_tasks_via_wire_protocol() {
    let gateway = Arc::new(AegisGateway::default());
    let handler = Arc::new(McpProtocolHandler::new(Arc::clone(&gateway)));

    handler
        .register_tools(vec![ToolDefinition {
            name: "fetch_data".to_string(),
            server: "api".to_string(),
            description: "Fetch API data".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec![],
            when_to_use: "Fetch data".to_string(),
            tags: vec![],
        }])
        .await;

    let req = json!({
        "jsonrpc": "2.0",
        "id": 102,
        "method": "gateway_plan_tasks",
        "params": {
            "plan_id": "test-wire-plan",
            "title": "API Pipeline",
            "steps": [{
                "id": "step-1",
                "tool": "fetch_data",
                "arguments": {},
                "depends_on": []
            }]
        }
    });

    let resp_str = handler
        .handle_message(&req.to_string())
        .await
        .unwrap()
        .unwrap();
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["result"]["valid"], true);
    assert_eq!(resp["result"]["step_count"], 1);
    assert_eq!(resp["result"]["human_approval_required"], false);
}
