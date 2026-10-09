# Aegis Gateway: Agentic Workflow & Coding Guidelines

> [!IMPORTANT]
> This file defines non-negotiable rules and constraints for all AI coding agents working on `aegis-gateway`.

## 1. Core Architectural Laws (SOLID & Enterprise-First)

1. **Dependency Inversion Principle (DIP)**:
   - High-level orchestrators (`AegisGateway`) must depend ONLY on abstractions defined in `src/core/` (`Arc<dyn Trait>`).
   - Never couple high-level routing directly to concrete database drivers, local file caches, or third-party client structs.

2. **Interface Segregation Principle (ISP)**:
   - Keep traits small, cohesive, and granular (`DistributedCache`, `DistributedRateLimiter`, `DistributedCircuitBreaker`, `DlpPipeline`, `PolicyEngine`).
   - Do not create monolithic "God" traits.

3. **Zero Unsafe Code**:
   - `#![deny(unsafe_code)]` is strictly enforced at the crate root.
   - Any commit introducing an `unsafe` block without explicit, approved cryptographic/FFI justification will be rejected.

4. **Production Code Hygiene**:
   - Zero bare `unwrap()` or `expect()` in production library code (`src/`).
   - All errors must map into typed variants of `AegisError`.

## 2. Agentic Workflow Lifecycle & Enforcement

In `agy` (Antigravity CLI), agent compliance is governed through a multi-tier pipeline:

1. **Rule Ingestion**: As you work in `/root/projects/aegis-gateway`, this `AGENTS.md` file is automatically injected into your context to constrain coding behavior.
2. **On-Demand Skills**: When auditing or refactoring, activate the skills in `.agents/skills/`:
   - `enterprise-readiness-auditor`
   - `mcp-protocol-governor`
   - `solid-code-reviewer`
3. **Automated Stop Hook Gate**:
   - Configured in `.agents/hooks.json`.
   - When any coding agent attempts to finish a turn or complete a task (`model_stop`), `agy` executes `./scripts/agy_stop_hook.sh`.
   - If `scripts/governance-check.sh` fails, the agent is **hard-blocked from stopping** and forced to remediate all violations before completing the task.

## 3. Mandatory Verification Checklist

Before reporting task completion to the user, run:
```bash
bash scripts/governance-check.sh
```
All 4 steps (SOLID review, 6 Enterprise Pillars, MCP conformance, and `cargo test`) must report `[PASS]` (100% compliance).
