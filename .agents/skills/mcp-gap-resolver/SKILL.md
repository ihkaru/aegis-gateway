---
name: mcp-gap-resolver
description: Autonomous skill to detect functional and operational gaps between legacy MCP gateways (MikkoParkkola/mcp-gateway, docker/mcp-gateway, microsoft/mcp-gateway) and Aegis Gateway via live GitHub research, formulate new phase roadmaps, and implement enterprise-grade solutions with graduated test suites and zero mocks.
---

# MCP Gap Resolver & Autonomous Phase Scaffolder

This skill equips coding agents with an end-to-end autonomous workflow to:
1. Conduct real-time research into open and closed GitHub issues of legacy/first-generation MCP gateways, primarily [MikkoParkkola/mcp-gateway](https://github.com/MikkoParkkola/mcp-gateway) (the canonical Rust reference gateway under PolyForm Noncommercial license), as well as `docker/mcp-gateway` and `microsoft/mcp-gateway`.
2. **Deduplication Invariant (Zero Re-evaluation of Resolved Issues)**: Cross-reference detected issues against `docs/ISSUES_PARITY_MATRIX.md` to automatically filter out all previously resolved bugs. Agents MUST NOT waste tokens re-searching or re-evaluating issues already resolved in Aegis Gateway.
3. Formulate and scaffold structured roadmap milestones (`docs/roadmap/NN_PHASE_NN_*.md`).
4. Implement genuine, enterprise-grade Rust code adhering to SOLID principles, `< 350 lines per file`, `#![deny(unsafe_code)]`, and zero mock closures under a 100% permissive MIT license.
5. Generate graduated test suites verifying real operational behavior under both golden and adversarial conditions.
6. Enforce zero-mock integrity via automated audit gates.
7. Record resolved issues in the permanent enterprise parity ledger (`docs/ISSUES_PARITY_MATRIX.md`).

---

## 🛠️ The 7-Step Repeatable Workflow

```
+-------------------+      +--------------------+      +--------------------+
| 1. Live Research  | ---> | 2. Gap Taxonomy &  | ---> | 3. Phase & Test    |
|    (--exclude-res)|      |    Prioritization  |      |    Scaffolding     |
+-------------------+      +--------------------+      +--------------------+
                                                                  |
+-------------------+      +--------------------+      +----------v---------+
| 7. Enterprise     | <--- | 6. Zero-Mock & CI  | <--- | 4. Interface-First |
|    Ledger Record  |      |    Verification    |      |    Implementation  |
+-------------------+      +--------------------+      +--------------------+
```

---

### Step 1: Live GitHub Research & Gap Detection

Query live GitHub issues from target repositories, automatically excluding already resolved items:

```bash
# 1. Scan only unresolved gaps (filters out issues cataloged in docs/ISSUES_PARITY_MATRIX.md)
python3 .agents/skills/mcp-gap-resolver/scripts/research_github_gaps.py \
  --repo MikkoParkkola/mcp-gateway,docker/mcp-gateway,microsoft/mcp-gateway \
  --state all \
  --limit 30 \
  --exclude-resolved \
  --local-root /root/projects/aegis-gateway

# 2. Targeted search for specific keywords excluding resolved issues
python3 .agents/skills/mcp-gap-resolver/scripts/research_github_gaps.py \
  --repo MikkoParkkola/mcp-gateway \
  --query "leak" \
  --limit 15 \
  --exclude-resolved

# 3. Audit current parity coverage and ledger integrity
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
- `SANDBOX_SEC`: Hermetic Sandboxing, Zero-Knowledge Credential Brokerage & Egress Firewall.

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
4. Outputs ready-to-use template snippets for `docs/ISSUES_PARITY_MATRIX.md`.

---

### Step 4: Interface-First Implementation Guidelines

When implementing the code to resolve the gaps:

1. **Define Core Traits First**:
   - Define trait abstractions in `src/core/` before writing structs.
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
3. **Security & Refusal Integrity**: Verify explicit refusal audit events (`AuditAction::Blocked`) with rejection rule metadata.

Execute test suite:
```bash
cargo nextest run --test phase<NN>_<title>_test
```

---

### Step 6: Zero-Mock Integrity & Governance Verification

Run the comprehensive audit battery:

```bash
# 1. Verify 100% zero-mock compliance and structured waivers
bash scripts/audit_mock_detection.sh

# 2. Verify parity matrix integrity & test proof
python3 .agents/skills/mcp-gap-resolver/scripts/audit_gap_coverage.py

# 3. Verify all test suites and line limits in parallel
bash scripts/governance-check.sh
```

All commands must exit with status `0`.

---

### Step 7: Enterprise Resolution Ledger Recording Standard

Once a phase is implemented and verified, agents **MUST** execute the four-tier recording protocol:

1. **Update Master Parity Matrix (`docs/ISSUES_PARITY_MATRIX.md`)**:
   Add resolved issues to `## 2. Comprehensive Issue Resolution Ledger`:
   ```markdown
   | `<UpstreamRepo>` | [#{NUM}](https://github.com/<UpstreamRepo>/issues/{NUM}) | `{CAT}` | {Deficit Summary} | {Aegis Resolution} | `{src_path}` | `{test_path}` |
   ```
2. **Synchronize Roadmap**:
   Update `docs/roadmap/00_ROADMAP_OVERVIEW.md` and the phase file to `Completed`.
3. **Run Audit Verifier**:
   Execute `python3 .agents/skills/mcp-gap-resolver/scripts/audit_gap_coverage.py`. It confirms that the newly registered source and test files exist on disk with 100% integrity.
4. **Semantic Git Commit & Push**:
   ```bash
   git add -A
   git commit -m "feat(phase-<NN>): resolve <gaps> (fixes <repo>#<NUM>)"
   git push origin main
   ```
