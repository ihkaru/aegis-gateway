// SPDX-License-Identifier: MIT

use serde_json::json;
use aegis_gateway::policy::AbacPolicyEngine;
use aegis_gateway::core::types::{CallerContext, TenantId};
use aegis_gateway::discovery::{default_enterprise_catalog, ExecutionPlanner, GoalPlanner};

#[tokio::test]
async fn test_autonomous_goal_deconstruction_and_multi_tool_planning() {
    let catalog = default_enterprise_catalog();
    let planner = ExecutionPlanner::new();
    let policy = AbacPolicyEngine::new();
    let caller = CallerContext {
        tenant_id: TenantId::new("tenant-test"),
        subject: "agent-planner".to_string(),
        roles: vec!["developer".to_string()],
        department: Some("Eng".to_string()),
        client_ip: None,
        session_id: "sess-plan-1".to_string(),
    };

    // Multi-step complex objective in Indonesian vernacular
    let goal = "Ambil data transaksi ledger database, simpan berkas ke cloud storage, lalu kirim pesan alert ke tim ops";

    let params = json!({ "goal": goal });
    let result = GoalPlanner::process_planning_request(
        &params,
        &catalog,
        &planner,
        &policy,
        &caller,
    )
    .await
    .expect("Goal planning must return successful result");

    assert!(result["valid"].as_bool().unwrap());
    assert_eq!(result["step_count"].as_u64().unwrap(), 3);

    let plan = &result["plan"];
    let steps = plan["steps"].as_array().expect("Plan must contain steps");
    assert_eq!(steps.len(), 3);

    // Step 1: query_postgresql_ledger
    assert_eq!(steps[0]["tool"].as_str().unwrap(), "query_postgresql_ledger");
    assert!(steps[0]["depends_on"].as_array().unwrap().is_empty());

    // Step 2: upload_to_storage (depends on step_1)
    assert_eq!(steps[1]["tool"].as_str().unwrap(), "upload_to_storage");
    assert_eq!(steps[1]["depends_on"][0].as_str().unwrap(), "step_1");

    // Step 3: dispatch_slack_alert (depends on step_2)
    assert_eq!(steps[2]["tool"].as_str().unwrap(), "dispatch_slack_alert");
    assert_eq!(steps[2]["depends_on"][0].as_str().unwrap(), "step_2");
}

#[tokio::test]
async fn test_english_commerce_invoicing_goal_planning() {
    let catalog = default_enterprise_catalog();
    let planner = ExecutionPlanner::new();
    let policy = AbacPolicyEngine::new();
    let caller = CallerContext {
        tenant_id: TenantId::new("tenant-test"),
        subject: "agent-commerce".to_string(),
        roles: vec!["developer".to_string()],
        department: Some("Billing".to_string()),
        client_ip: None,
        session_id: "sess-plan-2".to_string(),
    };

    let goal = "Register new paying customer on Stripe, charge customer credit card for order, then dispatch invoice email to customer inbox";

    let params = json!({ "goal": goal });
    let result = GoalPlanner::process_planning_request(
        &params,
        &catalog,
        &planner,
        &policy,
        &caller,
    )
    .await
    .expect("Goal planning must succeed");

    assert!(result["valid"].as_bool().unwrap());
    let steps = result["plan"]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 3);

    // Verify correct tool identification across financial tools
    assert_eq!(steps[0]["tool"].as_str().unwrap(), "stripe_create_customer");
    assert_eq!(steps[1]["tool"].as_str().unwrap(), "stripe_charge_card");
    assert_eq!(steps[2]["tool"].as_str().unwrap(), "sendgrid_send_invoice_email");

    // Verify automated data piping
    let charge_args = &steps[1]["arguments"];
    assert!(charge_args["customer_id"].as_str().unwrap().contains("{{step_1.output.customer_id}}"));
}
