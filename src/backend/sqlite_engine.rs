// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::core::database::{SqlEngine, SqlQueryResult};
use crate::core::error::{PropylonError, PropylonResult};

/// Real Local SQLite database engine implementing the SqlEngine abstraction contract (OCP & LSP)
#[derive(Debug, Clone)]
pub struct LocalSqliteEngine {
    db_path: PathBuf,
}

impl Default for LocalSqliteEngine {
    fn default() -> Self {
        Self::with_default_path()
    }
}

impl LocalSqliteEngine {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
        let path = db_path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self { db_path: path }
    }

    pub fn with_default_path() -> Self {
        Self::new("data/propylon_local.db")
    }
}

#[async_trait]
impl SqlEngine for LocalSqliteEngine {
    async fn execute(&self, query: &str) -> PropylonResult<SqlQueryResult> {
        let script = r#"
import sqlite3, json, sys, os
db_path = sys.argv[1]
query = sys.argv[2]
if os.path.dirname(db_path):
    os.makedirs(os.path.dirname(db_path), exist_ok=True)
conn = sqlite3.connect(db_path)
cur = conn.cursor()
try:
    cur.execute(query)
    q_upper = query.strip().upper()
    if q_upper.startswith(("SELECT", "PRAGMA", "EXPLAIN")):
        cols = [d[0] for d in cur.description] if cur.description else []
        rows = [dict(zip(cols, r)) for r in cur.fetchall()]
        print(json.dumps({"status": "SUCCESS", "type": "SELECT", "row_count": len(rows), "rows": rows}))
    else:
        conn.commit()
        print(json.dumps({"status": "SUCCESS", "type": "MUTATION", "rows_affected": cur.rowcount}))
except Exception as e:
    print(json.dumps({"status": "ERROR", "error": str(e)}))
"#;

        let db_str = self.db_path.to_string_lossy().to_string();
        let q_str = query.to_string();

        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("python3")
                .arg("-c")
                .arg(script)
                .arg(&db_str)
                .arg(&q_str)
                .output()
        })
        .await
        .map_err(|e| PropylonError::Internal(format!("Failed to join SQLite task: {e}")))?
        .map_err(|e| PropylonError::Internal(format!("Failed to execute SQLite process: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(SqlQueryResult::Error {
                error: stderr.trim().to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Value = serde_json::from_str(stdout.trim()).map_err(|e| {
            PropylonError::Internal(format!("Invalid output from SQLite engine: {e}"))
        })?;

        match parsed.get("type").and_then(|t| t.as_str()) {
            Some("SELECT") => {
                let count = parsed.get("row_count").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
                let rows = parsed
                    .get("rows")
                    .and_then(|r| r.as_array())
                    .cloned()
                    .unwrap_or_default();
                Ok(SqlQueryResult::Select {
                    row_count: count,
                    rows,
                })
            }
            Some("MUTATION") => {
                let affected = parsed
                    .get("rows_affected")
                    .and_then(|a| a.as_i64())
                    .unwrap_or(0);
                Ok(SqlQueryResult::Mutation {
                    rows_affected: affected,
                })
            }
            _ => {
                let err = parsed
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown SQLite execution error");
                Ok(SqlQueryResult::Error {
                    error: err.to_string(),
                })
            }
        }
    }

    async fn is_healthy(&self) -> bool {
        self.execute("SELECT 1;").await.is_ok()
    }

    fn engine_name(&self) -> &'static str {
        "sqlite-hermetic-local"
    }
}
