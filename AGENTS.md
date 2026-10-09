# Aegis Gateway: Agentic Workflow & Coding Guidelines

> [!IMPORTANT]
> This file defines non-negotiable rules and constraints for all AI coding agents working on `aegis-gateway`.

## 1. Core Architectural Laws (SOLID & Enterprise-First)

1. **SOLID Principles Held Tightly**:
   - **SRP**: Single Responsibility per file. Keep modules focused and domain-bounded.
   - **OCP**: Open for extension via new trait implementations; closed for modification of core orchestration logic.
   - **LSP**: All trait implementations (e.g., in-memory dev mocks vs Redis cluster backends) must satisfy identical contracts.
   - **ISP**: Small, cohesive, and segregated traits (`DistributedCache`, `DistributedRateLimiter`, `DistributedCircuitBreaker`, `QuotaEngine`).
   - **DIP**: High-level orchestrators (`AegisGateway`) depend ONLY on abstractions in `src/core/` (`Arc<dyn Trait>`).

2. **Interface-First Principle**:
   - Zero concrete implementation before abstraction.
   - Core contracts MUST be defined in `src/core/` as pure traits before any storage driver or client adapter is created.

3. **Dependency Injection (DI) Pattern**:
   - Constructors must receive trait objects (`Arc<dyn Trait>`) or bounded generics.
   - Never instantiate concrete drivers directly inside gateway pipelines or business services.

4. **Strict File Length Constraint (<= 350 lines)**:
   - **NO SINGLE FILE MAY EXCEED 350 LINES OF CODE** (`wc -l <= 350`) across `src/`, `tests/`, `scripts/`, and `.agents/`.
   - Files approaching 300 lines must be proactively refactored and decomposed into submodules.

5. **Zero Unsafe Code & Memory Safety**:
   - `#![deny(unsafe_code)]` is strictly enforced at the crate root (`src/lib.rs` and `src/main.rs`).
   - Zero bare `unwrap()` or `expect()` in production library code (`src/`).
   - All errors must map into typed variants of `AegisError`.

## 2. Agentic Workflow Lifecycle & Enforcement

In `agy` (Antigravity CLI), agent compliance is governed through a multi-tier pipeline:

1. **Rule Ingestion**: `AGENTS.md` and `/root/GEMINI.md` constraints are automatically enforced.
2. **On-Demand Skills**: When auditing or refactoring, activate the skills in `.agents/skills/`:
   - `enterprise-readiness-auditor`
   - `mcp-enterprise-gap-auditor`
   - `mcp-protocol-governor`
   - `solid-code-reviewer`
3. **Automated Stop Hook Gate**:
   - Configured in `.agents/hooks.json`.
   - When any coding agent attempts to finish a turn (`Stop` event), `agy` executes `./scripts/agy_stop_hook.sh`.
   - If `scripts/governance-check.sh` fails, the agent is **hard-blocked from stopping** and forced to remediate all violations before completing the task.

## 3. Mandatory Verification Checklist

Before reporting task completion to the user, run:
```bash
bash scripts/governance-check.sh
```
All 5 steps must report `[PASS]` (100% compliance).
