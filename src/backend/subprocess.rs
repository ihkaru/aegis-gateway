// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
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
        Self::build_command_for_backend("default", command, args, explicit_env)
    }

    /// Build hermetic Command with per-backend package manager cache isolation
    pub fn build_command_for_backend(
        backend_name: &str,
        command: &str,
        args: &[String],
        explicit_env: &HashMap<String, String>,
    ) -> Command {
        let mut cmd = Command::new(command);

        // 1. Enforce hermetic isolation: clear ALL host environment variables (OWASP LLM08)
        cmd.env_clear();

        // 2. Inject safe minimum OS primitives
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

        #[cfg(windows)]
        {
            for key in ["USERPROFILE", "APPDATA", "LOCALAPPDATA", "TEMP", "TMP", "SYSTEMROOT"] {
                if let Ok(v) = std::env::var(key) {
                    cmd.env(key, v);
                }
            }
        }

        // 3. Isolated package-manager caches to prevent concurrent corruption (Issue #622)
        let first_word = command.split_whitespace().next().unwrap_or(command);
        let prog = first_word.rsplit('/').next().unwrap_or(first_word).trim();
        let safe_name = Self::sanitize_component(backend_name);
        let base_cache = std::env::temp_dir().join("aegis-cache");

        if matches!(prog, "npx" | "npm" | "pnpm" | "yarn" | "bunx") && !explicit_env.contains_key("npm_config_cache") {
            cmd.env("npm_config_cache", base_cache.join("npm").join(&safe_name));
        }
        if matches!(prog, "uv" | "uvx") && !explicit_env.contains_key("UV_CACHE_DIR") {
            cmd.env("UV_CACHE_DIR", base_cache.join("uv").join(&safe_name));
        }
        if matches!(prog, "pip" | "pipx") && !explicit_env.contains_key("PIP_CACHE_DIR") {
            cmd.env("PIP_CACHE_DIR", base_cache.join("pip").join(&safe_name));
        }

        // 4. Inject explicitly declared backend configuration variables
        for (k, v) in explicit_env {
            cmd.env(k, v);
        }

        // 5. Pass arguments as typed argv array (CWE-78 defense: zero shell interpolation)
        cmd.args(args);

        // 6. Configure non-blocking pipes
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit()); // Diagnostic logs flow safely to stderr

        // 7. Anti-zombie guarantee: terminate child cleanly on drop
        cmd.kill_on_drop(true);

        cmd
    }

    fn sanitize_component(name: &str) -> String {
        let cleaned: String = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        if cleaned.is_empty() {
            "unnamed".to_string()
        } else {
            cleaned
        }
    }
}

#[async_trait]
impl BackendTransport for HermeticSubprocessBackend {
    async fn start(&self) -> AegisResult<()> {
        let command_str = self.config.command.as_ref().ok_or_else(|| {
            AegisError::Internal(format!("No command specified for backend '{}'", self.config.name))
        })?;

        let mut cmd = Self::build_command_for_backend(
            &self.config.name,
            command_str,
            &self.config.args,
            &self.config.env,
        );
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
            // Stage 1: Explicitly drop stdin to signal EOF to the child process (Issue #2530)
            drop(proc.stdin);

            // Stage 2: Time-bounded wait for child exit before escalating to SIGKILL
            let wait_fut = proc.child.wait();
            let timeout = std::time::Duration::from_millis(1500);

            if (tokio::time::timeout(timeout, wait_fut).await).is_err() {
                // Stage 3: Escalation to SIGKILL on deadline expiration
                let _ = proc.child.kill().await;
            }

            // Stage 4: Drain any remaining stdout bytes to avoid corrupted buffers (Issue #2573)
            let mut buf = String::new();
            let _ = tokio::time::timeout(
                std::time::Duration::from_millis(500),
                proc.reader.read_to_string(&mut buf),
            )
            .await;
        }
        Ok(())
    }
}
