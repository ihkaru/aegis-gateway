---
name: mcp-gap-resolver
description: Autonomous skill to detect functional and operational gaps between legacy MCP gateways (docker/mcp-gateway, microsoft/mcp-gateway) and Aegis Gateway via live GitHub research, formulate new phase roadmaps, and implement enterprise-grade solutions with graduated test suites and zero mocks.
---

# MCP Gap Resolver & Autonomous Phase Scaffolder

This skill equips coding agents with an end-to-end autonomous workflow to:
1. Conduct real-time research into open and closed GitHub issues of legacy/first-generation MCP gateways (`docker/mcp-gateway`, `microsoft/mcp-gateway`, etc.).
2. Cross-reference detected issues against Aegis Gateway to identify unaddressed edge cases, bugs, and functional gaps.
3. Formulate and scaffold structured roadmap milestones (`docs/roadmap/NN_PHASE_NN_*.md`).
4. Implement genuine, enterprise-grade Rust code adhering to SOLID principles, `< 350 lines per file`, `#![deny(unsafe_code)]`, and zero mock closures.
5. Generate graduated test suites verifying real operational behavior under both golden and adversarial conditions.
6. Enforce zero-mock integrity via automated audit gates.

---

## 🛠️ The 7-Step Repeatable Workflow

```
+-------------------+      +--------------------+      +--------------------+
| 1. Live Research  | ---> | 2. Gap Taxonomy &  | ---> | 3. Phase & Test    |
|    GitHub Issues  |      |    Prioritization  |      |    Scaffolding     |
+-------------------+      +--------------------+      +--------------------+
                                                                  |
+-------------------+      +--------------------+      +----------v---------+
| 6. Documentation  | <--- | 5. Zero-Mock & CI  | <--- | 4. Interface-First |
|    & Remote Push  |      |    Verification    |      |    Implementation  |
+-------------------+      +--------------------+      +--------------------+
```

---

### Step 1: Live GitHub Research & Gap Detection

Query live GitHub issues from target repositories and cross-reference with local Aegis code:

```bash
# General scan of latest 30 issues across docker and microsoft repositories
python3 .agents/skills/mcp-gap-resolver/scripts/research_github_gaps.py \
  --repo docker/mcp-gateway,microsoft/mcp-gateway \
  --state all \
  --limit 30 \
  --local-root /root/projects/aegis-gateway

# Targeted scan for specific operational keywords (e.g. leak, timeout, refusal, proxy)
python3 .agents/skills/mcp-gap-resolver/scripts/research_github_gaps.py \
  --repo docker/mcp-gateway \
  --query "leak" \
  --limit 15

# Audit local test coverage of canonical complaints
python3 .agents/skills/mcp-gap-resolver/scripts/audit_gap_coverage.py
```

Inspect output tables for items flagged with **`P0 (Critical)`** or **`[GAP]`**.

---

### Step 2: Gap Classification & Prioritization

Classify identified issues using [`references/GAP_TAXONOMY.md`](references/GAP_TAXONOMY.md):
- `PROT`: Protocol Conformance (framing, header validation, clean Stdio).
- `PROC`: Subprocess Lifecycle (zombie prevention, env leakage, cache isolation).
- `SEC`: Security & Governance (payload ABAC, refusal audit logging, DLP).
- `OPS`: Operational HA & Resilience (circuit breakers, timeouts, drain).
- `DISC`: Discovery & Planning (progressive disclosure, DAG cycle checks).

Group 2 to 4 related gaps into a coherent theme for the next implementation phase.

---

### Step 3: Automated Phase & Test Scaffolding

Scaffold the new phase documents and test skeleton in one command:

```bash
python3 .agents/skills/mcp-gap-resolver/scripts/scaffold_phase.py \
  --phase-num <NEXT_PHASE_NUM> \
  --title "<Descriptive Phase Title>" \
  --issues "docker/mcp-gateway#<NUM>,microsoft/mcp-gateway#<NUM>"
```

This automatically generates:
1. `docs/roadmap/<NN>_PHASE_<NN>_<TITLE>.md` (Milestone specification).
2. `tests/phase<NN>_<title>_test.rs` (Integration test suite skeleton).
3. Updates `docs/roadmap/00_ROADMAP_OVERVIEW.md` ledger.

---

### Step 4: Interface-First Implementation Guidelines

When implementing the code to resolve the gaps:

1. **Define Core Traits First**:
   - If introducing new abstractions, define them in `src/core/` before writing structs.
   - Example: `pub trait ToolOutcomeVerifier: Send + Sync { ... }`.
2. **Implement Production Drivers**:
   - Place drivers in appropriate submodules (`src/transport/`, `src/lifecycle/`, `src/security/`).
   - Invert dependencies: orchestrators accept `Arc<dyn Trait>` via constructors.
3. **Hard Constraint Checklist**:
   - **File Length**: `wc -l <= 350` on every new and modified `.rs` file.
   - **Zero Unsafe**: Never introduce `unsafe` blocks (`#![deny(unsafe_code)]`).
   - **Zero Mocks in `src/`**: Never use mock closures or fake response strings in production paths.

---

### Step 5: Test Suite & Edge Case Verification

Implement graduated test cases in `tests/phase<NN>_<title>_test.rs`:
1. **Happy Path (Golden Standard)**: Validate expected success behavior under compliant input.
2. **Negative & Boundary**: Validate handling of malformed inputs, timeouts, or abrupt socket drops.
3. **Security & Refusal Integrity**: If testing access denials or refused calls (e.g. Issue #591), verify that audit logs record explicit `AuditAction::Blocked` events with cause metadata.

Execute test suite:
```bash
cargo test --test phase<NN>_<title>_test
```

---

### Step 6: Zero-Mock Integrity & Governance Verification

Run the comprehensive audit battery:

```bash
# 1. Verify 100% zero-mock compliance
bash scripts/audit_mock_detection.sh

# 2. Verify all test suites and line limits
bash scripts/governance-check.sh

# 3. Verify AGY stop hook permits task completion
bash scripts/agy_stop_hook.sh
```

All commands must exit with status `0`.

---

### Step 7: Documentation & Remote Synchronization

1. Update `README.md`:
   - Add new persona capability or gap parity highlight.
   - Add the new test command to the graduated testing list.
   - Update the Enterprise Roadmap table with the completed phase.
2. Update `/root/GEMINI.md`:
   - Add checklist entry under the mandatory roadmap section.
3. Commit and push:
   ```bash
   git add .
   git commit -m "feat(phase-<NN>): resolve <gaps> (<issue-refs>)"
   git push origin main
   ```
