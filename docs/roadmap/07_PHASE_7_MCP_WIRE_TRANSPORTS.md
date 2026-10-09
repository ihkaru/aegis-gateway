# Phase 7: MCP Wire Protocol & Dual Ingress Transports (Stdio & SSE)

> **Milestone Tag**: `v0.8.0-wire-transports`  
> **Status**: `Planned`  
> **Target Standard**: Model Context Protocol Specification (2024-11-05 & 2025-11-25)

---

## 1. Objectives

Provide standard JSON-RPC 2.0 wire protocol handling and dual client ingress channels (Stdio and Streamable HTTP/SSE). This enables direct, plug-and-play connectivity with standard AI clients (Claude Desktop, Cursor IDE, VS Code, Goose, and custom autonomous agents) without requiring custom API SDKs.

---

## 2. Architecture & Contracts

Defined in `src/core/transport.rs` (to be created):

```rust
#[async_trait]
pub trait IngressTransport: Send + Sync {
    async fn run(&self) -> AegisResult<()>;
}

pub trait WireProtocolHandler: Send + Sync {
    fn handle_message(&self, raw_json: &str) -> AegisResult<Option<String>>;
}
```

### JSON-RPC 2.0 Invariants

1. **Protocol Handshake**:
   - `initialize`: Client negotiates protocol version, server capability matrix (`tools`, `resources`, `prompts`), and gateway identity.
   - `notifications/initialized`: Completes the handshake sequence.
   - `ping`: Returns empty response `{}` for liveness heartbeat.
2. **Standard Tool Methods**:
   - `tools/list`: Returns list of available tools (either native tools or progressive meta-tools).
   - `tools/call`: Dispatches tool invocation through Aegis Zero-Trust pipeline (ABAC -> DLP -> Quota -> Execution -> Sanitization -> Audit).
3. **Transport Conformance**:
   - **Stdio Transport**: Reads newline-delimited JSON-RPC from `stdin` and writes formatted JSON-RPC to `stdout`. Logs and diagnostics are strictly isolated to `stderr` to avoid protocol corruption.
   - **Streamable HTTP / SSE Transport**: Supports `/sse` subscription and `/message` POST endpoints.

---

## 3. Milestones & Checklist

- [ ] **7.1 JSON-RPC 2.0 Message Models & Framing**: Standard `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcNotification`, and canonical error codes (`-32600`, `-32601`, `-32602`, `-32603`).
- [ ] **7.2 Standard MCP Protocol Handshake Engine**: Support `initialize`, `notifications/initialized`, and `ping` compliant with MCP 2024-11-05 and 2025-11-25.
- [ ] **7.3 Native Stdio Client Ingress (`--stdio`)**: Async stdin line reader and stdout writer enabling direct integration in `claude_desktop_config.json`.
- [ ] **7.4 Streamable HTTP / SSE Client Ingress (`--http <addr>`)**: SSE endpoint for event streams and HTTP POST endpoint for web/remote agent frameworks.
- [ ] **7.5 Meta-Tool & Direct Exposure Duality**: Support both compact meta-tools (`gateway_search_tools`, `gateway_invoke`) and direct transparent tool listing based on policy configuration.
