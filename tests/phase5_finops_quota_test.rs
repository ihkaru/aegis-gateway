// SPDX-License-Identifier: MIT

use aegis_gateway::core::error::AegisError;
use aegis_gateway::core::state::{BudgetManager, BudgetStatus, QuotaEngine, TenantBudget};
use aegis_gateway::core::types::TenantId;
use aegis_gateway::state::HardFreezeQuota;

#[tokio::test]
async fn test_phase5_realtime_metering_and_tenant_isolation() {
    let quota = HardFreezeQuota::new();
    let tenant_fin = TenantId::new("tenant-finance");
    let tenant_mkt = TenantId::new("tenant-marketing");

    // Configure distinct budgets
    quota
        .set_budget(TenantBudget {
            tenant_id: tenant_fin.clone(),
            department: "Finance".to_string(),
            monthly_limit_usd: 1000.0,
            current_spend_usd: 0.0,
            tokens_consumed: 0,
            hard_freeze_enabled: true,
        })
        .await
        .expect("Failed to set finance budget");

    quota
        .set_budget(TenantBudget {
            tenant_id: tenant_mkt.clone(),
            department: "Marketing".to_string(),
            monthly_limit_usd: 400.0,
            current_spend_usd: 0.0,
            tokens_consumed: 0,
            hard_freeze_enabled: true,
        })
        .await
        .expect("Failed to set marketing budget");

    // Record usage for Finance
    quota
        .record_usage(&tenant_fin, 50_000, 25.50)
        .await
        .expect("Failed to record finance usage");

    // Record usage for Marketing
    quota
        .record_usage(&tenant_mkt, 120_000, 60.00)
        .await
        .expect("Failed to record marketing usage");

    // Verify isolation
    let fin_b = quota.get_budget(&tenant_fin).await.unwrap().unwrap();
    assert_eq!(fin_b.tokens_consumed, 50_000);
    assert!((fin_b.current_spend_usd - 25.50).abs() < 1e-4);

    let mkt_b = quota.get_budget(&tenant_mkt).await.unwrap().unwrap();
    assert_eq!(mkt_b.tokens_consumed, 120_000);
    assert!((mkt_b.current_spend_usd - 60.00).abs() < 1e-4);
}

#[tokio::test]
async fn test_phase5_soft_warning_and_hard_freeze_cutoff() {
    let quota = HardFreezeQuota::new();
    let tenant = TenantId::new("tenant-autonomous-agent");

    // $100 budget
    quota
        .set_budget(TenantBudget {
            tenant_id: tenant.clone(),
            department: "R&D".to_string(),
            monthly_limit_usd: 100.0,
            current_spend_usd: 0.0,
            tokens_consumed: 0,
            hard_freeze_enabled: true,
        })
        .await
        .unwrap();

    // 1. Normal state ($50 spend = 50%)
    quota.record_spend(&tenant, 50.0).await.unwrap();
    let status_50 = quota.evaluate_status(&tenant).await.unwrap();
    match status_50 {
        BudgetStatus::Normal { percent_used } => assert!((percent_used - 50.0).abs() < 1e-3),
        other => panic!("Expected Normal status, got {:?}", other),
    }
    assert!(quota.check_budget(&tenant).await.unwrap());

    // 2. Soft warning trigger at 80% ($35 more = $85 spend = 85%)
    quota.record_spend(&tenant, 35.0).await.unwrap();
    let status_85 = quota.evaluate_status(&tenant).await.unwrap();
    match status_85 {
        BudgetStatus::Warning { percent_used, message } => {
            assert!((percent_used - 85.0).abs() < 1e-3);
            assert!(message.contains("80% threshold reached"));
        }
        other => panic!("Expected Warning status, got {:?}", other),
    }
    assert!(quota.check_budget(&tenant).await.unwrap());

    // 3. Hard cutoff freeze trigger at 100% ($15 more = $100 spend = 100%)
    quota
        .record_usage(&tenant, 10_000, 15.0)
        .await
        .expect("Reaching exactly 100% limit");
    let status_100 = quota.evaluate_status(&tenant).await.unwrap();
    match status_100 {
        BudgetStatus::Frozen { percent_used, .. } => assert!(percent_used >= 100.0),
        other => panic!("Expected Frozen status, got {:?}", other),
    }

    // check_budget must now return false to block upstream calls
    let budget_ok = quota.check_budget(&tenant).await.unwrap();
    assert!(!budget_ok, "check_budget must return false when hard limit reached");

    // Further record_spend or record_usage attempts must return BudgetFrozen error
    let freeze_err = quota.record_spend(&tenant, 1.0).await;
    match freeze_err {
        Err(AegisError::BudgetFrozen { reason, .. }) => {
            assert!(reason.contains("exhausted"));
        }
        other => panic!("Expected AegisError::BudgetFrozen, got {:?}", other),
    }
}

#[tokio::test]
async fn test_phase5_departmental_chargeback_and_prometheus_metrics() {
    let quota = HardFreezeQuota::new();

    // Setup multiple departments
    let t_eng1 = TenantId::new("eng-backend");
    let t_eng2 = TenantId::new("eng-frontend");
    let t_ops = TenantId::new("ops-infra");

    quota
        .set_budget(TenantBudget {
            tenant_id: t_eng1.clone(),
            department: "Engineering".to_string(),
            monthly_limit_usd: 500.0,
            current_spend_usd: 150.0,
            tokens_consumed: 300_000,
            hard_freeze_enabled: true,
        })
        .await
        .unwrap();

    quota
        .set_budget(TenantBudget {
            tenant_id: t_eng2.clone(),
            department: "Engineering".to_string(),
            monthly_limit_usd: 500.0,
            current_spend_usd: 250.0,
            tokens_consumed: 500_000,
            hard_freeze_enabled: true,
        })
        .await
        .unwrap();

    quota
        .set_budget(TenantBudget {
            tenant_id: t_ops.clone(),
            department: "Operations".to_string(),
            monthly_limit_usd: 200.0,
            current_spend_usd: 50.0,
            tokens_consumed: 100_000,
            hard_freeze_enabled: true,
        })
        .await
        .unwrap();

    // Generate chargeback report
    let report = quota.generate_chargeback_report().await.unwrap();
    assert_eq!(report.len(), 2);
    // Engineering should be #1 with $400 ($150 + $250) and 2 tenants
    assert_eq!(report[0].department, "Engineering");
    assert!((report[0].total_cost_usd - 400.0).abs() < 1e-3);
    assert_eq!(report[0].total_tokens, 800_000);
    assert_eq!(report[0].tenant_count, 2);

    // Operations should be #2 with $50
    assert_eq!(report[1].department, "Operations");
    assert!((report[1].total_cost_usd - 50.0).abs() < 1e-3);
    assert_eq!(report[1].tenant_count, 1);

    // Export Prometheus metrics
    let metrics = quota.render_prometheus_metrics().await.unwrap();
    assert!(metrics.contains("aegis_tenant_cost_usd_total{tenant=\"eng-backend\",department=\"Engineering\"} 150.0000"));
    assert!(metrics.contains("aegis_tenant_tokens_total{tenant=\"eng-frontend\",department=\"Engineering\"} 500000"));
    assert!(metrics.contains("aegis_tenant_budget_frozen{tenant=\"ops-infra\",department=\"Operations\"} 0"));
}
