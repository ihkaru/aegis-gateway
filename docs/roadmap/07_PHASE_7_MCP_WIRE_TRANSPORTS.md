# Phase 7: MCP Wire Protocol & Dual Ingress Transports (Stdio & Streamable HTTP)

> **Milestone Tag**: `v0.8.0-wire-transports`  
> **Status**: `Planned`  
> **Target Standard**: Model Context Protocol Specification (2024-11-05 & Streamable HTTP RFC 2025-03-26)

---

## 1. Objectives

Provide standard JSON-RPC 2.0 wire protocol handling and modern enterprise ingress channels:
1. **Native Stdio Client Ingress (`--stdio`)**: For desktop AI agents (Claude Desktop, Cursor IDE, VS Code, Goose).
2. **Streamable HTTP Ingress (`--http <addr>`)**: Adopting the modern unified single-endpoint architecture (`POST /mcp` with scoped SSE streams) to eliminate the dual-endpoint SSE routing failures common in enterprise Kubernetes ALBs.

---

## 2. Architecture & Contracts

Defined in `src/core/transport.rs`:

```rust
#[async_trait]
pub trait IngressTransport: Send + Sync {
    async fn run(&self) -> AegisResult<()>;
}

#[async_trait]
pub trait WireProtocolHandler: Send + Sync {
    async fn handle_message(&self, raw_json: &str) -> AegisResult<Option<String>>;
}
```

### Enterprise Transport Invariants & Mitigations

1. **Streamable HTTP vs Legacy Dual-SSE (Kubernetes ALB Traps)**:
   - *Legacy Fault*: Separate `/sse` (GET) and `/message` (POST) endpoints required stateful session affinity, frequently breaking in multi-pod Kubernetes behind AWS ALB / Cloudflare.
   - *Aegis Standard*: Unified Streamable HTTP endpoint (`POST /mcp`) supporting stateless chunked JSON-RPC and request-scoped SSE streaming per RFC 2025-03-26, with backward-compatible SSE fallback.
2. **Zero-Contamination Stdio Channel (Anti-Hang Invariant)**:
   - Any raw print to `stdout` corrupts the AI client's JSON parser, causing silent process death.
   - Aegis guarantees `stdout` is strictly reserved for framed JSON-RPC 2.0 messages; all diagnostic traces and errors are unconditionally directed to `stderr`.
3. **Canonical JSON-RPC 2.0 Error Taxonomy**:
   - Strict adherence to specification error codes:
     - `-32700`: Parse error (invalid JSON)
     - `-32600`: Invalid Request (missing jsonrpc version or method)
     - `-32601`: Method not found
     - `-32602`: Invalid params
     - `-32603`: Internal gateway error

---

## 3. Milestones & Checklist

- [ ] **7.1 JSON-RPC 2.0 Message Models & Framing**: Standard `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcNotification`, and canonical error taxonomy (`-32700..-32603`).
- [ ] **7.2 Standard MCP Protocol Handshake Engine**: Support `initialize`, `notifications/initialized`, and `ping` compliant with MCP 2024-11-05 & 2025-03-26.
- [ ] **7.3 Native Stdio Client Ingress (`--stdio`)**: Async stdin line reader and stdout writer with strict stderr-isolated tracing.
- [ ] **7.4 Enterprise Streamable HTTP Ingress**: Unified `POST /mcp` endpoint with streaming chunked transfer and legacy SSE backward compatibility.
- [ ] **7.5 Meta-Tool & Direct Exposure Duality**: Seamless support for both compact meta-tools (`gateway_search_tools`, `gateway_invoke`) and direct transparent tool listing.

