// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::error::PropylonResult;

/// Typed execution outcome of a relational SQL command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SqlQueryResult {
    Select { row_count: usize, rows: Vec<Value> },
    Mutation { rows_affected: i64 },
    Error { error: String },
}

impl SqlQueryResult {
    pub fn to_json_value(&self) -> Value {
        match self {
            Self::Select { row_count, rows } => serde_json::json!({
                "status": "SUCCESS",
                "type": "SELECT",
                "row_count": row_count,
                "rows": rows,
            }),
            Self::Mutation { rows_affected } => serde_json::json!({
                "status": "SUCCESS",
                "type": "MUTATION",
                "rows_affected": rows_affected,
            }),
            Self::Error { error } => serde_json::json!({
                "status": "ERROR",
                "error": error,
            }),
        }
    }
}

/// Interface-First Database Abstraction Contract (DIP & ISP)
#[async_trait]
pub trait SqlEngine: Send + Sync {
    /// Execute arbitrary SQL command asynchronously
    async fn execute(&self, query: &str) -> PropylonResult<SqlQueryResult>;

    /// Check if database file/connection is healthy
    async fn is_healthy(&self) -> bool;

    /// Descriptive engine identifier (e.g., "sqlite-hermetic", "postgresql-pool")
    fn engine_name(&self) -> &'static str;
}
