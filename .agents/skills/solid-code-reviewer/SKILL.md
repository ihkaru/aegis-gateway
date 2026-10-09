---
name: solid-code-reviewer
description: Strict architectural and code review skill enforcing SOLID design principles, Zero Unsafe Code, and clean trait abstractions in Rust.
---

# SOLID Code Reviewer Skill

This skill enforces strict software craftsmanship principles across the Rust codebase to prevent architectural decay, coupling, and hidden technical debt.

## SOLID Principles in Rust Implementation

1. **S - Single Responsibility Principle (SRP)**
   - Every module and struct must have exactly one reason to change.
   - Separate core traits (`core/`) from concrete database/network drivers (`state/`, `policy/`, `audit/`).
   - File lengths must remain bounded (ideally <= 600 LOC per module).

2. **O - Open/Closed Principle (OCP)**
   - Open for extension, closed for modification.
   - New state stores (e.g., Redis, DynamoDB), policy evaluators (e.g., OPA, Cedar), or audit sinks (e.g., Splunk, Datadog) must be added by implementing traits, never by hacking `match` statements across the core pipeline.

3. **L - Liskov Substitution Principle (LSP)**
   - Implementations of a trait (e.g., `InMemoryCache` vs `RedisCache`) must fulfill the exact same semantic contract.
   - A mock or in-memory implementation in tests must behave identically to the production clustered implementation.

4. **I - Interface Segregation Principle (ISP)**
   - Fine-grained traits rather than monolithic "God" traits.
   - Separate traits: `ToolDiscoverer`, `ToolInvoker`, `PolicyEvaluator`, `DataMasker`, `AuditSink`, `SkillRegistry`.
   - Consumers only depend on the interfaces they actually invoke.

5. **D - Dependency Inversion Principle (DIP)**
   - High-level gateway orchestrators depend on abstractions (`dyn Trait`), not concrete structs.
   - Concrete dependencies are injected at construction time (Constructor Dependency Injection).

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
