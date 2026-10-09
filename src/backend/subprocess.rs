// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

use crate::core::backend::{BackendConfig, BackendTransport};
use crate::core::error::{AegisError, AegisResult};
use crate::core::transport::{JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, INTERNAL_ERROR};

struct ProcessHandle {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

/// Hermetic subprocess backend with env_clear() isolation and anti-zombie kill_on_drop
pub struct HermeticSubprocessBackend {
    config: BackendConfig,
    handle: Arc<Mutex<Option<ProcessHandle>>>,
}

impl HermeticSubprocessBackend {
    pub fn new(config: BackendConfig) -> Self {
        Self {
            config,
            handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Build sanitized, hermetic Command without host environment variable leakage
    pub fn build_command(
        command: &str,
        args: &[String],
        explicit_env: &HashMap<String, String>,
    ) -> Command {
        let mut cmd = Command::new(command);

        // 1. Enforce hermetic isolation: clear ALL host environment variables (OWASP LLM08)
        cmd.env_clear();

        // 2. Inject only safe minimum OS primitives
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }
        if let Ok(home) = std::env::var("HOME") {
            cmd.env("HOME", home);
        }
        if let Ok(tmp) = std::env::var("TMPDIR") {
            cmd.env("TMPDIR", tmp);
        } else {
            cmd.env("TMPDIR", "/tmp");
        }

        // 3. Inject only explicitly declared backend configuration variables
        for (k, v) in explicit_env {
            cmd.env(k, v);
        }

        // 4. Pass arguments as typed argv array (CWE-78 defense: zero shell interpolation)
        cmd.args(args);

        // 5. Configure non-blocking pipes
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit()); // Diagnostic logs flow safely to stderr

        // 6. Anti-zombie guarantee: terminate child cleanly on drop
        cmd.kill_on_drop(true);

        cmd
    }
}

#[async_trait]
impl BackendTransport for HermeticSubprocessBackend {
    async fn start(&self) -> AegisResult<()> {
        let command_str = self.config.command.as_ref().ok_or_else(|| {
            AegisError::Internal(format!("No command specified for backend '{}'", self.config.name))
        })?;

        let mut cmd = Self::build_command(command_str, &self.config.args, &self.config.env);
        let mut child = cmd.spawn().map_err(|e| {
            AegisError::Internal(format!("Failed to spawn backend process '{}': {e}", self.config.name))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            AegisError::Internal(format!("Failed to open stdin for backend '{}'", self.config.name))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            AegisError::Internal(format!("Failed to open stdout for backend '{}'", self.config.name))
        })?;

        let mut guard = self.handle.lock().await;
        *guard = Some(ProcessHandle {
            child,
            stdin,
            reader: BufReader::new(stdout),
        });

        Ok(())
    }

    async fn send_request(&self, request: &JsonRpcRequest) -> AegisResult<JsonRpcResponse> {
        let mut guard = self.handle.lock().await;
        let proc = match guard.as_mut() {
            Some(p) => p,
            None => {
                return Ok(JsonRpcResponse::error(
                    request.id.clone().unwrap_or(JsonRpcId::Null),
                    JsonRpcError::new(INTERNAL_ERROR, format!("Backend '{}' is not running", self.config.name)),
                ));
            }
        };

        let req_json = serde_json::to_string(request)?;
        proc.stdin.write_all(req_json.as_bytes()).await?;
        proc.stdin.write_all(b"\n").await?;
        proc.stdin.flush().await?;

        let mut line = String::new();
        match proc.reader.read_line(&mut line).await {
            Ok(bytes) if bytes > 0 => {
                let resp: JsonRpcResponse = serde_json::from_str(line.trim()).map_err(|e| {
                    AegisError::Internal(format!(
                        "Invalid JSON-RPC response from backend '{}': {e}",
                        self.config.name
                    ))
                })?;
                Ok(resp)
            }
            _ => Ok(JsonRpcResponse::error(
                request.id.clone().unwrap_or(JsonRpcId::Null),
                JsonRpcError::new(
                    INTERNAL_ERROR,
                    format!("Backend '{}' closed pipe without response", self.config.name),
                ),
            )),
        }
    }

    async fn is_healthy(&self) -> bool {
        let mut guard = self.handle.lock().await;
        match guard.as_mut() {
            Some(proc) => proc.child.try_wait().map(|status| status.is_none()).unwrap_or(false),
            None => false,
        }
    }

    async fn stop(&self) -> AegisResult<()> {
        let mut guard = self.handle.lock().await;
        if let Some(mut proc) = guard.take() {
            let _ = proc.child.kill().await;
        }
        Ok(())
    }
}
