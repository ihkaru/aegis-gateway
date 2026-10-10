//! Phase 15 Integration Test Suite: Runtime DAG Expression Interpolation Pipeline and Hybrid Semantic RRF Tool Retrieval Engine
//! Target Gaps: Aegis/Scale#101 (Runtime Data Piping), Aegis/Scale#102 (Hybrid Semantic Search at Scale)

#![deny(unsafe_code)]

use serde_json::json;
use std::time::Instant;

use aegis_gateway::core::types::{DisclosureTier, ToolDefinition};
use aegis_gateway::discovery::{
    DagPipelineEngine, ExecutionPlan, HybridSearchEngine, PlanStep,
};

#[tokio::test]
async fn test_runtime_dag_expression_interpolation_and_multi_hop_pipeline() {
    let engine = DagPipelineEngine::new();

    let plan = ExecutionPlan {
        plan_id: "plan-pipe-01".to_string(),
        title: "Database Backup to Object Storage with Ops Notification".to_string(),
        steps: vec![
            PlanStep {
                id: "step_1".to_string(),
                tool: "backup_database".to_string(),
                server: Some("db_cluster".to_string()),
                arguments: json!({ "database": "prod_users" }),
                depends_on: vec![],
                description: "Create database backup".to_string(),
            },
            PlanStep {
                id: "step_2".to_string(),
                tool: "upload_to_storage".to_string(),
                server: Some("mediavault".to_string()),
                arguments: json!({
                    "source_file": "{{step_1.backup_id}}",
                    "file_size": "{{step_1.size_bytes}}",
                    "bucket": "secure-backups"
                }),
                depends_on: vec!["step_1".to_string()],
                description: "Upload snapshot to object storage".to_string(),
            },
            PlanStep {
                id: "step_3".to_string(),
                tool: "send_slack_alert".to_string(),
                server: Some("ops_notify".to_string()),
                arguments: json!({
                    "channel": "#infra-alerts",
                    "message": "Snapshot {{step_1.backup_id}} stored safely at {{step_2.s3_uri}}"
                }),
                depends_on: vec!["step_2".to_string()],
                description: "Notify engineering team".to_string(),
            },
        ],
    };

    // Step runner function simulating realistic backend responses
    let step_runner = |_step_id: String, tool: String, args: serde_json::Value| async move {
        match tool.as_str() {
            "backup_database" => Ok(json!({
                "backup_id": "bak-prod-20261010.sql.gz",
                "size_bytes": 10485760
            })),
            "upload_to_storage" => {
                let file = args.get("source_file").and_then(|v| v.as_str()).unwrap_or("");
                assert_eq!(file, "bak-prod-20261010.sql.gz", "Dynamic interpolation must match step 1 output");
                Ok(json!({
                    "s3_uri": format!("s3://secure-backups/{}", file),
                    "status": "uploaded"
                }))
            }
            "send_slack_alert" => {
                let msg = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
                assert!(msg.contains("bak-prod-20261010.sql.gz"));
                assert!(msg.contains("s3://secure-backups/bak-prod-20261010.sql.gz"));
                Ok(json!({ "delivered": true }))
            }
            other => panic!("Unexpected tool: {}", other),
        }
    };

    let result = engine
        .execute_pipeline(&plan, step_runner)
        .await
        .expect("Pipeline execution must succeed");

    assert!(result.success, "Pipeline execution must be successful");
    assert_eq!(result.total_steps, 3);
    assert_eq!(result.completed_steps, 3);
    assert_eq!(result.step_results.len(), 3);
    assert!(result.step_outputs.contains_key("step_1"));
    assert!(result.step_outputs.contains_key("step_2"));
    assert!(result.step_outputs.contains_key("step_3"));
}

#[tokio::test]
async fn test_runtime_dag_fail_closed_on_missing_interpolation_key() {
    let engine = DagPipelineEngine::new();

    let plan = ExecutionPlan {
        plan_id: "plan-pipe-fail".to_string(),
        title: "Faulty Interpolation Plan".to_string(),
        steps: vec![
            PlanStep {
                id: "step_A".to_string(),
                tool: "create_record".to_string(),
                server: Some("crm".to_string()),
                arguments: json!({ "name": "Alice" }),
                depends_on: vec![],
                description: "Create customer".to_string(),
            },
            PlanStep {
                id: "step_B".to_string(),
                tool: "send_email".to_string(),
                server: Some("mail".to_string()),
                arguments: json!({ "target_id": "{{step_A.non_existent_key}}" }),
                depends_on: vec!["step_A".to_string()],
                description: "Send welcome mail".to_string(),
            },
        ],
    };

    let step_runner = |_step_id: String, tool: String, _args: serde_json::Value| async move {
        if tool == "create_record" {
            Ok(json!({ "customer_id": "cust-123" }))
        } else {
            Ok(json!({ "sent": true }))
        }
    };

    let result = engine
        .execute_pipeline(&plan, step_runner)
        .await
        .expect("Pipeline call should return result envelope");

    assert!(!result.success, "Pipeline must fail-closed on missing interpolated key");
    assert_eq!(result.completed_steps, 1);
    let step_b_res = result.step_results.iter().find(|s| s.step_id == "step_B").unwrap();
    assert!(!step_b_res.success);
    assert!(step_b_res.error.as_ref().unwrap().contains("Interpolation failed"));
}

#[test]
fn test_hybrid_search_rrf_semantic_retrieval_across_colloquial_queries() {
    let mut engine = HybridSearchEngine::new();

    let tools = vec![
        ToolDefinition {
            name: "query_database_ledger".to_string(),
            server: "db_cluster".to_string(),
            description: "Execute SQL statements and retrieve transaction records".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec!["query".to_string()],
            when_to_use: "When financial or tabular data is requested".to_string(),
            tags: vec!["database".to_string(), "sql".to_string(), "tabel".to_string()],
        },
        ToolDefinition {
            name: "upload_to_storage".to_string(),
            server: "mediavault".to_string(),
            description: "Store documents and images into persistent S3 cloud bucket".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec!["path".to_string()],
            when_to_use: "When preserving files or uploads".to_string(),
            tags: vec!["s3".to_string(), "file".to_string(), "berkas".to_string()],
        },
        ToolDefinition {
            name: "dispatch_slack_alert".to_string(),
            server: "ops_notify".to_string(),
            description: "Broadcast incident message to engineering room".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec!["channel".to_string()],
            when_to_use: "When notifying teams".to_string(),
            tags: vec!["slack".to_string(), "alert".to_string(), "pesan".to_string()],
        },
        ToolDefinition {
            name: "destructive_drop_table".to_string(),
            server: "db_cluster".to_string(),
            description: "Permanently remove a table and all its rows".to_string(),
            input_schema: json!({ "type": "object" }),
            required_params: vec!["table".to_string()],
            when_to_use: "When discarding schema".to_string(),
            tags: vec!["drop".to_string(), "delete".to_string(), "hapus".to_string()],
        },
    ];

    engine.index_tools("default", tools);

    // Query 1: Indonesian colloquial "simpan berkas ke cloud" (No exact match for English "upload_to_storage")
    let res1 = engine.search("simpan berkas ke cloud", DisclosureTier::L0, 3);
    assert!(!res1.tools.is_empty());
    assert_eq!(res1.tools[0].name, "upload_to_storage");

    // Query 2: Colloquial "hapus tabel data"
    let res2 = engine.search("hapus tabel data", DisclosureTier::L0, 3);
    assert!(!res2.tools.is_empty());
    assert_eq!(res2.tools[0].name, "destructive_drop_table");

    // Query 3: Ops alert "kirim pesan alert"
    let res3 = engine.search("kirim pesan alert", DisclosureTier::L0, 3);
    assert!(!res3.tools.is_empty());
    assert_eq!(res3.tools[0].name, "dispatch_slack_alert");
}

#[test]
fn test_hybrid_search_scale_resilience_with_100_tools() {
    let mut engine = HybridSearchEngine::new();

    // Generate 100 enterprise tools
    let mut catalog = Vec::new();
    for i in 1..=100 {
        let category = match i % 4 {
            0 => "database",
            1 => "storage",
            2 => "notification",
            _ => "ai_analytics",
        };
        catalog.push(ToolDefinition {
            name: format!("{}_service_tool_{}", category, i),
            server: format!("{}_cluster", category),
            description: format!("Perform specialized operations on {} node #{}", category, i),
            input_schema: json!({ "type": "object" }),
            required_params: vec!["id".to_string()],
            when_to_use: format!("When handling {} tasks", category),
            tags: vec![category.to_string(), format!("node_{}", i)],
        });
    }

    engine.index_tools("cluster", catalog);

    let t0 = Instant::now();
    let res = engine.search("database query node 24", DisclosureTier::L0, 5);
    let latency_us = t0.elapsed().as_micros();

    assert_eq!(res.tools.len(), 5);
    assert!(res.tools[0].name.contains("database"));
    assert!(latency_us < 15_000, "Search over 100 tools must finish in < 15ms (actual: {}us)", latency_us);
}
