# Phase 51: Dynamic Tool Catalog Scoping & Least-Privilege Exposure

> **Milestone Tag**: `v1.51.0-catalog_scoping_least_privilege`  
> **Status**: `In-Progress`  
> **Target Enterprise Gaps & Standards**:
> - Dynamic `tools/list` response filtering based on caller identity, department, and clearance level.
> - Elimination of LLM context window token waste by hiding unauthorized tools.
> - Protection against reconnaissance and information disclosure by untrusted agents.
> - Integration with Progressive Disclosure (L0 Metadata -> L1 Lightweight -> L2 Full Schema).

---

## 1. Problem Statement & Motivation

When an enterprise has 100+ tools registered across 20 MCP servers:
1. Sending all 100 tools to an LLM consumes 30,000+ context tokens per turn, inflating FinOps costs.
2. Exposing sensitive financial, HR, or infrastructure tool definitions to unauthorized employees violates least-privilege principles.
3. The gateway must filter tool discovery responses dynamically before transmitting them to the caller.

---

## 2. Technical Architecture & Trait Contracts

```rust
#[async_trait]
pub trait CatalogScopingProvider: Send + Sync {
    /// Filters discovered tools dynamically for the given authenticated principal
    async fn scope_catalog(&self, principal: &PrincipalAttributes, raw_tools: Vec<ToolDefinition>) -> AegisResult<Vec<ToolDefinition>>;
}
```

### Key Guarantees:
- **Zero Information Leakage**: Unauthorized tools are completely omitted from `tools/list`.
- **Cached Scoping**: Fast cache lookup per principal role/clearance hash.
- **Consistent Execution Rejection**: Even if an agent crafts a direct invocation for an unscoped tool, the ABAC engine denies it immediately.
