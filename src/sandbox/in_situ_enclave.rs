// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use crate::core::data_governance::{InSituDataEnclave, InSituExecutionResult};
use crate::core::error::{AegisError, AegisResult};
use crate::core::sandbox::{CodeSandboxEngine, ExecutionLanguage, SandboxExecutionRequest};
use crate::core::types::CallerContext;

/// Server-side Sandboxed In-Situ Analytical Enclave (Zero-Egress Processing)
pub struct SandboxedInSituEnclave {
    sandbox: Arc<dyn CodeSandboxEngine>,
}

impl SandboxedInSituEnclave {
    pub fn new(sandbox: Arc<dyn CodeSandboxEngine>) -> Self {
        Self { sandbox }
    }
}

#[async_trait]
impl InSituDataEnclave for SandboxedInSituEnclave {
    async fn execute_query(
        &self,
        query: &str,
        resource_path: &str,
        caller: &CallerContext,
    ) -> AegisResult<InSituExecutionResult> {
        let start = Instant::now();

        // Enforce honest filesystem pre-check: reject non-existent paths immediately
        if !Path::new(resource_path).exists() {
            return Err(AegisError::ResourceNotFound(format!(
                "In-situ dataset not found on server filesystem: '{}'",
                resource_path
            )));
        }

        let script = format!(
            r#"
import sys, os, json, sqlite3, csv

resource_path = "{resource_path}"
query = """{query}"""

if not os.path.exists(resource_path):
    print(json.dumps({{"error": f"Resource not found: {{resource_path}}"}}))
    sys.exit(2)

raw_bytes = os.path.getsize(resource_path)
columns = []
rows = []
row_count = 0

try:
    is_sqlite = resource_path.endswith(('.db', '.sqlite', '.sqlite3'))
    is_csv = resource_path.endswith(('.csv', '.tsv'))
    if not is_sqlite and not is_csv:
        try:
            with open(resource_path, 'r', encoding='utf-8', errors='ignore') as f_sniff:
                first = f_sniff.readline()
                if ',' in first or '\t' in first:
                    is_csv = True
        except Exception:
            pass

    if is_sqlite:
        conn = sqlite3.connect(resource_path)
        cur = conn.cursor()
        cur.execute(query)
        columns = [desc[0] for desc in cur.description] if cur.description else ["result"]
        fetched = cur.fetchmany(25)
        rows = [[str(c) for c in r] for r in fetched]
        row_count = len(rows)
        conn.close()
    elif is_csv:
        delimiter = '\t' if resource_path.endswith('.tsv') else ','
        with open(resource_path, 'r', encoding='utf-8', errors='ignore') as f:
            reader = csv.reader(f, delimiter=delimiter)
            header = next(reader, None)
            columns = header if header else ["col_0"]
            samples = []
            count = 0
            for r in reader:
                count += 1
                if len(samples) < 15:
                    samples.append(r)
            row_count = count
            rows = samples
    else:
        with open(resource_path, 'r', encoding='utf-8', errors='ignore') as f:
            lines = [l.strip() for l in f if l.strip()]
            row_count = len(lines)
            columns = ["record_id", "preview"]
            rows = [[str(idx + 1), l[:80]] for idx, l in enumerate(lines[:15])]

    header_line = "| " + " | ".join(columns) + " |"
    divider_line = "| " + " | ".join(["---"] * len(columns)) + " |"
    row_lines = ["| " + " | ".join(r) + " |" for r in rows[:15]]
    summary = "\n".join([header_line, divider_line] + row_lines)

    output = {{
        "columns": columns,
        "row_count": row_count,
        "summary_table": summary,
        "raw_bytes_read": raw_bytes
    }}
    print(json.dumps(output))
except Exception as e:
    print(json.dumps({{"error": str(e)}}))
    sys.exit(1)
"#,
            resource_path = resource_path,
            query = query.replace('"', "\\\"")
        );

        let req = SandboxExecutionRequest {
            language: ExecutionLanguage::Python,
            code: script,
            timeout_secs: Some(30),
            services: vec![],
            env_vars: std::collections::HashMap::new(),
            tenant_id: Some(caller.tenant_id.clone()),
            caller_id: Some(caller.subject.clone()),
        };

        let result = self.sandbox.execute(&req).await?;
        let duration_ms = start.elapsed().as_millis() as u64;

        if !result.success {
            return Err(AegisError::Internal(format!(
                "In-situ analytics execution failed (code {}): {}",
                result.exit_code, result.stderr
            )));
        }

        let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())
            .map_err(|e| AegisError::Internal(format!("Failed to parse in-situ JSON: {e}")))?;

        if let Some(err_msg) = parsed.get("error").and_then(|e| e.as_str()) {
            return Err(AegisError::Internal(format!("In-situ engine error: {err_msg}")));
        }

        let columns: Vec<String> = parsed
            .get("columns")
            .and_then(|c| c.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["result".to_string()]);

        let row_count = parsed.get("row_count").and_then(|r| r.as_u64()).unwrap_or(0) as usize;
        let summary_table = parsed
            .get("summary_table")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let raw_bytes_read = parsed.get("raw_bytes_read").and_then(|b| b.as_u64()).unwrap_or(0);
        let egress_bytes_returned = summary_table.len() as u64;

        Ok(InSituExecutionResult {
            success: true,
            row_count,
            columns,
            summary_table,
            duration_ms,
            raw_bytes_read,
            egress_bytes_returned,
        })
    }
}
