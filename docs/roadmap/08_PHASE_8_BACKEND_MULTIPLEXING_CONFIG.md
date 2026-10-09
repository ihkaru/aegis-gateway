# Phase 8: Backend Process Spawning, Upstream Multiplexing & Config Engine

> **Milestone Tag**: `v0.9.0-backend-multiplexing`  
> **Status**: `Planned`  
> **Target Standard**: Hermetic Process Sandboxing (OWASP LLM08), Anti-Command-Injection (CWE-78), Zero-Zombie Process Hygiene

---

## 1. Objectives

Enable Aegis Gateway to act as a secure, production-grade reverse proxy for arbitrary backend MCP servers (Node.js/npx, Python, Go, Docker, or remote HTTP/SSE servers). Load server topologies declaratively from configuration files (`aegis.yaml`), enforce hermetic subprocess sandboxing, manage process lifecycles without leaking host credentials, and dynamically aggregate tool catalogs.

---

## 2. Architecture & Contracts

Defined in `src/core/backend.rs`:

```rust
#[async_trait]
pub trait BackendTransport: Send + Sync {
    async fn start(&self) -> AegisResult<()>;
    async fn send_request(&self, request: &JsonRpcRequest) -> AegisResult<JsonRpcResponse>;
    async fn is_healthy(&self) -> bool;
    async fn stop(&self) -> AegisResult<()>;
}

#[async_trait]
pub trait BackendRegistry: Send + Sync {
    async fn register_backend(&self, name: &str, transport: Arc<dyn BackendTransport>) -> AegisResult<()>;
    async fn get_backend(&self, name: &str) -> AegisResult<Arc<dyn BackendTransport>>;
    async fn discover_all_tools(&self) -> AegisResult<Vec<ToolDefinition>>;
}
```

### Enterprise Subprocess Sandboxing Invariants

1. **Hermetic Environment Variable Sanitization (Anti-Secret Exfiltration)**:
   - *Threat (OWASP LLM08)*: Third-party MCP packages (npm/pip) running locally blindly inherit host environment variables containing `AWS_SECRET_ACCESS_KEY`, database passwords, and API keys.
   - *Aegis Mitigation*: Subprocesses are launched with strict `env_clear()`. Only safe OS primitives (`PATH`, `HOME`, `TMPDIR`) and explicitly declared backend environment variables are injected.
2. **Anti-Command-Injection Execution (CWE-78 Defense)**:
   - Command arguments are strictly passed as typed string vectors `Vec<String>` without shell interpolation (`sh -c` or `bash -c` string formatting is strictly prohibited).
3. **Anti-Zombie Process Group Guarantee**:
   - Subprocesses are assigned to dedicated process groups with `kill_on_drop(true)` to ensure that gateway termination, client disconnection, or server restart terminates all child processes cleanly without leaving orphaned zombie processes.
4. **Resilient Failure Isolation**:
   - If a backend crashes or hangs, the circuit breaker isolates that specific backend while the rest of the gateway and other MCP tools remain fully available.

---

## 3. Milestones & Checklist

- [ ] **8.1 Declarative Configuration Parser (`aegis.yaml`)**: Parse backend topologies supporting `command`, `args`, `env`, and remote `url` formats compatible with Claude Desktop and legacy `gateway.yaml`.
- [ ] **8.2 Hermetic Subprocess Spawner & Lifecycle Supervisor**: Spawn asynchronous child processes with strict `env_clear()`, `kill_on_drop(true)`, and piped standard IO.
- [ ] **8.3 Remote HTTP/SSE Backend Connector**: Forward tool calls to remote network-attached MCP endpoints with connection pooling and timeouts.
- [ ] **8.4 Automated Catalog Aggregation**: Interrogate all registered backends via `tools/list` upon startup, map to Aegis `ToolDefinition`, and populate the registry dynamically.
- [ ] **8.5 Full End-to-End Operational Pipeline**: Complete transparent bidirectional proxying uniting Client Ingress, Enterprise Control Plane, and Upstream Backends.

