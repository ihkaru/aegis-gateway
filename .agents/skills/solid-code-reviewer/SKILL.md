---
name: solid-code-reviewer
description: Strict architectural and code review skill enforcing SOLID design principles, Interface-First, Dependency Injection, <=350 lines per file, Zero Unsafe Code, and clean trait abstractions in Rust.
---

# SOLID Code Reviewer Skill

This skill enforces strict software craftsmanship principles across the Rust codebase to prevent architectural decay, coupling, and hidden technical debt.

## Core Architectural Invariants

1. **Strict File Length Constraint (<= 350 lines)**:
   - **No single file may exceed 350 lines of code** (`wc -l <= 350`).
   - If a file approaches 300 lines, it must be proactively refactored into focused submodules.

2. **Interface-First Principle**:
   - Zero concrete implementation before abstraction.
   - Core contracts MUST be defined in `src/core/` as pure asynchronous traits before any concrete adapter or storage driver is written.

3. **Dependency Injection (DI) Pattern**:
   - High-level orchestrators (`AegisGateway`) and pipeline services must receive trait objects (`Arc<dyn Trait>`) or bounded generics via constructor injection.
   - Concrete structs must NEVER be instantiated directly inside business services.

## SOLID Principles in Rust Implementation

1. **S - Single Responsibility Principle (SRP)**
   - Every module and struct must have exactly one reason to change.
   - Separate core traits (`core/`) from concrete database/network drivers (`state/`, `policy/`, `audit/`).

2. **O - Open/Closed Principle (OCP)**
   - Open for extension, closed for modification.
   - New state stores (e.g., Redis, DynamoDB), policy evaluators (e.g., OPA, Cedar), or audit sinks (e.g., Splunk, Datadog) must be added by implementing traits, never by hacking core logic.

3. **L - Liskov Substitution Principle (LSP)**
   - Implementations of a trait (e.g., `InMemoryStateBackend` vs `RedisStateBackend`) fulfill the identical contract.

4. **I - Interface Segregation Principle (ISP)**
   - Fine-grained traits rather than monolithic "God" traits (`DistributedCache`, `DistributedRateLimiter`, `DistributedCircuitBreaker`, `QuotaEngine`).

5. **D - Dependency Inversion Principle (DIP)**
   - High-level gateway orchestrators depend on abstractions (`dyn Trait`), not concrete structs.

## Rust Code Quality Standards

- `#![deny(unsafe_code)]` at crate root.
- No bare `unwrap()` or `expect()` in production codepaths (use `Result<T, AegisError>` and `?`).
- Pure asynchronous I/O via Tokio runtime.
- Proper use of `Arc<dyn Trait + Send + Sync>` for thread-safe shared references.

## Review Execution

Run the SOLID compliance check:
```bash
bash .agents/skills/solid-code-reviewer/scripts/audit_solid.sh
```
