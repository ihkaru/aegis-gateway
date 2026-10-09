# Phase 13: Reusable Stream Buffer, OAuth Protected Resource Metadata & Process Reaper

> **Milestone Tag**: `v1.13.0-stream-oauth-reaper`  
> **Status**: `Completed`  
> **Standards Reference**: RFC 8707 / OAuth 2.0 Protected Resource Metadata, Model Context Protocol (MCP 2024-11-05), JSON-RPC 2.0, POSIX Process Groups

---

## 1. Problem Statement & Motivation
Comparative audit against live GitHub issues of legacy MCP gateways (`microsoft/mcp-gateway`, `docker/mcp-gateway`) revealed four critical operational failure modes:
1. **Stream Consumption Panics (microsoft/mcp-gateway#48)**: Multi-stage inspection (DLP, ABAC, and auditing) exhausted request body streams, triggering *"The stream was already consumed. It cannot be read again"*.
2. **Missing OAuth Protected Resource Metadata (microsoft/mcp-gateway#17 & #20, docker/mcp-gateway#474)**: VS Code MCP extension and enterprise clients require RFC discovery (`/.well-known/oauth-protected-resource`), failing handshakes without it.
3. **Subprocess Container & Zombie Leaks (docker/mcp-gateway#483)**: High concurrent tool volume left orphaned background processes and leaked container handles upon client disconnection.
4. **Unsigned MCP Wire Messages (docker/mcp-gateway#453)**: Responses routed through gateways lacked cryptographic message signing (`x-mcp-signature`), preventing end-to-end non-repudiation.

---

## 2. Technical Architecture & Trait Contracts

### 2.1 Reusable Stream Buffer (`src/transport/buffer.rs`)
- `ReusableStreamBuffer` wraps immutable `Arc<[u8]>` with configurable capacity caps.
- `fork_reader()` generates independent zero-copy `std::io::Cursor` streams, enabling unlimited sequential and concurrent inspection passes across DLP, ABAC, and backend dispatch without stream exhaustion.

### 2.2 OAuth 2.0 Protected Resource Metadata (`src/policy/oauth_metadata.rs`)
- `ProtectedResourceMetadata` struct implementing RFC 8707 / OAuth 2.0 discovery specification.
- Exposes `/.well-known/oauth-protected-resource` route on `LiveHttpServer` returning resource URI, authorization servers, supported scopes, and bearer token methods with HTTP 200 OK.

### 2.3 Concurrency-Safe Process Group Reaper (`src/backend/reaper.rs`)
- `ProcessGroupReaper` maintains concurrency-safe registry of active child PIDs (`Arc<Mutex<HashSet<u32>>>`).
- Tracks child processes across async tasks; `terminate_pid()` and `reap_all()` cleanly terminate lingering PIDs with zero unsafe code and zero zombie processes left behind.

### 2.4 Wire Message Signer & Verifier (`src/transport/signer.rs`)
- `McpMessageSigner` computes HMAC-SHA256 signature tokens formatted as `t=<timestamp>,v1=<signature>`.
- `verify_signature()` validates message authenticity and rejects expired payloads exceeding clock skew limits (> 300s).

---

## 3. Empirical Verification Plan
- Integration test suite: `tests/phase13_stream_buffer_reuse_oauth_resource_metadata_and_process_reaper_test.rs` (4/4 tests passing).
- Zero-mock compliance: `bash scripts/audit_mock_detection.sh` (100% PASS).
- Hard line constraint: `wc -l <= 350` across all files.
