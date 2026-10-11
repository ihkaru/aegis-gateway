// SPDX-License-Identifier: MIT
//! Phase 43: Real In-Situ Analytical Enclave Engine Test Suite
//! Validates real server-side dataset querying, SQLite/CSV parsing, and zero-egress verification without mocks.

use std::io::Write;
use std::process::Command;
use std::sync::Arc;

use aegis_gateway::audit::StructuredAuditLogger;
use aegis_gateway::core::data_governance::InSituDataEnclave;
use aegis_gateway::core::types::{CallerContext, TenantId};
use aegis_gateway::policy::EnvSecretStore;
use aegis_gateway::sandbox::{EgressFilterEngine, HermeticProcessSandbox, SandboxedInSituEnclave, VaultCredentialBroker};

fn make_test_caller() -> CallerContext {
    CallerContext {
        tenant_id: TenantId::new("enterprise-analytics"),
        subject: "auditor@enterprise.internal".to_string(),
        roles: vec!["analyst".to_string()],
        department: Some("DataGovernance".to_string()),
        client_ip: None,
        session_id: "sess_test_analytics_43".to_string(),
    }
}

fn create_test_enclave() -> SandboxedInSituEnclave {
    let audit = Arc::new(StructuredAuditLogger::new());
    let egress = Arc::new(EgressFilterEngine::new());
    let secrets = Arc::new(EnvSecretStore::new());
    let broker = Arc::new(VaultCredentialBroker::new(secrets));
    let sandbox = Arc::new(HermeticProcessSandbox::new(egress, broker, audit));
    SandboxedInSituEnclave::new(sandbox)
}

#[tokio::test]
async fn test_phase43_real_sqlite_analytical_query() {
    let enclave = create_test_enclave();
    let caller = make_test_caller();

    let tmp = tempfile::Builder::new()
        .suffix(".sqlite")
        .tempfile()
        .expect("temp sqlite db");
    let path_str = tmp.path().to_str().unwrap().to_string();

    // Initialize real SQLite database schema and rows using python3 sqlite3 module
    let init_py = format!(
        "import sqlite3; conn = sqlite3.connect('{}'); cur = conn.cursor(); \
         cur.execute('CREATE TABLE financial_ledger (id INTEGER PRIMARY KEY, department TEXT, amount REAL, currency TEXT)'); \
         cur.executemany('INSERT INTO financial_ledger VALUES (?, ?, ?, ?)', [(1, 'Engineering', 45000.0, 'USD'), (2, 'Security', 62000.0, 'USD'), (3, 'Platform', 38000.0, 'USD')]); \
         conn.commit(); conn.close()",
        path_str
    );
    let status = Command::new("python3")
        .arg("-c")
        .arg(init_py)
        .status()
        .expect("python3 sqlite init should run");
    assert!(status.success(), "sqlite init must succeed");

    let query = "SELECT department, amount, currency FROM financial_ledger WHERE amount > 40000.0";
    let res = enclave
        .execute_query(query, &path_str, &caller)
        .await
        .expect("analytical query should succeed");

    assert!(res.success);
    assert_eq!(res.columns, vec!["department", "amount", "currency"]);
    assert_eq!(res.row_count, 2);
    assert!(res.summary_table.contains("Engineering"));
    assert!(res.summary_table.contains("Security"));
    assert!(!res.summary_table.contains("Platform"));
    assert!(res.raw_bytes_read > 1000);
    assert!(res.egress_bytes_returned < 1000);
}

#[tokio::test]
async fn test_phase43_real_csv_dataset_aggregation() {
    let enclave = create_test_enclave();
    let caller = make_test_caller();

    let tmp = tempfile::Builder::new()
        .suffix(".csv")
        .tempfile()
        .expect("temp csv");
    let mut file = tmp.as_file();
    writeln!(file, "timestamp,user_id,action,status").unwrap();
    for i in 1..=500 {
        writeln!(file, "2026-10-11T12:00:00Z,user_{},export_report,SUCCESS", i).unwrap();
    }
    file.flush().unwrap();
    let path_str = tmp.path().to_str().unwrap().to_string();

    let res = enclave
        .execute_query("INSPECT", &path_str, &caller)
        .await
        .expect("csv inspection should succeed");

    assert!(res.success);
    assert_eq!(res.columns, vec!["timestamp", "user_id", "action", "status"]);
    assert_eq!(res.row_count, 500);
    assert!(res.raw_bytes_read > 15_000);
    assert!(res.egress_bytes_returned < 2048);
}

#[tokio::test]
async fn test_phase43_missing_resource_honest_failure() {
    let enclave = create_test_enclave();
    let caller = make_test_caller();

    let err = enclave
        .execute_query("SELECT * FROM missing", "/tmp/non_existent_dataset_98765.db", &caller)
        .await
        .expect_err("missing dataset must return honest error");

    assert!(err.to_string().contains("not found on server filesystem"));
}

#[tokio::test]
async fn test_phase43_invalid_sql_syntax_handling() {
    let enclave = create_test_enclave();
    let caller = make_test_caller();

    let tmp = tempfile::Builder::new()
        .suffix(".sqlite")
        .tempfile()
        .expect("temp sqlite db");
    let path_str = tmp.path().to_str().unwrap().to_string();

    let err = enclave
        .execute_query("MALFORMED SQL STATEMENT SYNTAX ERROR", &path_str, &caller)
        .await
        .expect_err("malformed query must return error");

    assert!(err.to_string().contains("In-situ engine error") || err.to_string().contains("failed"));
}
