// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::core::audit::{AuditAction, AuditEvent, AuditSink};
use crate::core::error::{AegisError, AegisResult};
use crate::core::sandbox::{
    CodeSandboxEngine, CredentialBroker, EgressFirewall, ExecutionLanguage,
    SandboxExecutionRequest, SandboxExecutionResult,
};
use crate::core::types::CallerContext;

/// Hermetic Process Execution Sandbox Driver
pub struct HermeticProcessSandbox {
    egress_firewall: Arc<dyn EgressFirewall>,
    credential_broker: Arc<dyn CredentialBroker>,
    audit_sink: Arc<dyn AuditSink>,
    default_timeout_secs: u64,
    max_output_bytes: usize,
}

impl HermeticProcessSandbox {
    pub fn new(
        egress_firewall: Arc<dyn EgressFirewall>,
        credential_broker: Arc<dyn CredentialBroker>,
        audit_sink: Arc<dyn AuditSink>,
    ) -> Self {
        Self {
            egress_firewall,
            credential_broker,
            audit_sink,
            default_timeout_secs: 30,
            max_output_bytes: 524_288, // 512 KB
        }
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.default_timeout_secs = timeout_secs;
        self
    }

    fn compute_sha256(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        result.iter().fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{:02x}", b);
            acc
        })
    }

    fn resolve_runner(&self, lang: ExecutionLanguage) -> (&'static str, &'static str) {
        match lang {
            ExecutionLanguage::Python => ("python3", "main.py"),
            ExecutionLanguage::Bash => ("bash", "script.sh"),
            ExecutionLanguage::JavaScript => ("node", "main.js"),
        }
    }
}

#[async_trait]
impl CodeSandboxEngine for HermeticProcessSandbox {
    async fn is_available(&self) -> bool {
        tokio::process::Command::new("python3")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    async fn execute(&self, req: &SandboxExecutionRequest) -> AegisResult<SandboxExecutionResult> {
        let code_hash = Self::compute_sha256(&req.code);
        let start = Instant::now();

        // 1. Preflight Code Egress & SSRF Inspection
        if let Err(e) = self.egress_firewall.check_code_egress(&req.code).await {
            let caller = CallerContext {
                tenant_id: req.tenant_id.clone().unwrap_or_else(|| crate::core::types::TenantId::new("anonymous")),
                subject: req.caller_id.clone().unwrap_or_else(|| "anonymous".to_string()),
                roles: vec!["agent".to_string()],
                department: None,
                client_ip: None,
                session_id: uuid::Uuid::new_v4().to_string(),
            };
            self.audit_sink
                .emit(&AuditEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    caller,
                    action: AuditAction::PolicyEvaluated {
                        allowed: false,
                        reason: Some(e.to_string()),
                    },
                    target_resource: "sandbox:egress_firewall".to_string(),
                    payload_hash_sha256: code_hash.clone(),
                    metadata: serde_json::json!({
                        "violation": "EgressFirewallRefusal",
                        "error": e.to_string(),
                        "code_hash": code_hash
                    }),
                })
                .await?;
            return Err(e);
        }

        // 2. Zero-Knowledge Credential Brokerage
        let injected_envs = self.credential_broker.broker_credentials(&req.services).await?;
        let injected_keys: Vec<String> = injected_envs.keys().cloned().collect();

        // 3. Ephemeral Temp Directory Isolation
        let temp_dir = tempfile::tempdir()
            .map_err(|e| AegisError::Internal(format!("Failed to create ephemeral sandbox dir: {e}")))?;
        let (binary, filename) = self.resolve_runner(req.language);
        let script_path: PathBuf = temp_dir.path().join(filename);

        tokio::fs::write(&script_path, &req.code)
            .await
            .map_err(|e| AegisError::Internal(format!("Failed to write script into sandbox: {e}")))?;

        // 4. Hermetic Command Spawner with env_clear()
        let mut cmd = tokio::process::Command::new(binary);
        cmd.arg(&script_path);
        cmd.current_dir(temp_dir.path());
        cmd.env_clear();

        // Inject strictly necessary base environment
        cmd.env("PATH", "/usr/local/bin:/usr/bin:/bin");
        cmd.env("HOME", temp_dir.path());
        cmd.env("TMPDIR", temp_dir.path());
        cmd.env("LANG", "C.UTF-8");
        cmd.env("PYTHONUNBUFFERED", "1");

        for (k, v) in &req.env_vars {
            cmd.env(k, v);
        }
        for (k, v) in &injected_envs {
            cmd.env(k, v);
        }
        cmd.kill_on_drop(true);

        // 5. Execution with Hard Timeout
        let timeout_secs = req.timeout_secs.unwrap_or(self.default_timeout_secs);
        let timeout_duration = Duration::from_secs(timeout_secs);

        let (exit_code, raw_stdout, raw_stderr, success) = match tokio::time::timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                let code = output.status.code().unwrap_or(1);
                let out_str = String::from_utf8_lossy(&output.stdout).to_string();
                let err_str = String::from_utf8_lossy(&output.stderr).to_string();
                (code, out_str, err_str, output.status.success())
            }
            Ok(Err(e)) => {
                (1, String::new(), format!("Subprocess launch error: {e}"), false)
            }
            Err(_) => {
                (-1, String::new(), format!("Execution timed out after {timeout_secs}s"), false)
            }
        };

        // 6. Output Truncation & Credential Redaction Scrubbing
        let truncated_stdout = if raw_stdout.len() > self.max_output_bytes {
            format!("{}... [TRUNCATED {} BYTES]", &raw_stdout[..self.max_output_bytes], raw_stdout.len() - self.max_output_bytes)
        } else {
            raw_stdout
        };

        let truncated_stderr = if raw_stderr.len() > self.max_output_bytes {
            format!("{}... [TRUNCATED {} BYTES]", &raw_stderr[..self.max_output_bytes], raw_stderr.len() - self.max_output_bytes)
        } else {
            raw_stderr
        };

        let clean_stdout = self.credential_broker.redact_secrets(&truncated_stdout).await;
        let clean_stderr = self.credential_broker.redact_secrets(&truncated_stderr).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        // 7. Non-Repudiation Cryptographic Audit Emission
        let caller = CallerContext {
            tenant_id: req.tenant_id.clone().unwrap_or_else(|| crate::core::types::TenantId::new("anonymous")),
            subject: req.caller_id.clone().unwrap_or_else(|| "anonymous".to_string()),
            roles: vec!["agent".to_string()],
            department: None,
            client_ip: None,
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        let attestation = format!("sha256:{}:exit_{}:dur_{}ms", code_hash, exit_code, duration_ms);

        self.audit_sink
            .emit(&AuditEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                caller,
                action: AuditAction::ToolInvoked,
                target_resource: format!("sandbox:{:?}", req.language),
                payload_hash_sha256: code_hash.clone(),
                metadata: serde_json::json!({
                    "language": format!("{:?}", req.language),
                    "exit_code": exit_code,
                    "duration_ms": duration_ms,
                    "success": success,
                    "services": req.services,
                    "attestation": attestation
                }),
            })
            .await?;

        Ok(SandboxExecutionResult {
            success,
            exit_code,
            stdout: clean_stdout,
            stderr: clean_stderr,
            duration_ms,
            code_hash_sha256: code_hash,
            credentials_injected: injected_keys,
            attestation: Some(attestation),
        })
    }
}
