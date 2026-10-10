#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo "    AEGIS GATEWAY: UNIFIED ENTERPRISE GOVERNANCE RUNNER     "
echo "============================================================"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "Step 0: Running Production Zero-Mock Integrity Audit..."
bash scripts/audit_mock_detection.sh
echo ""

echo "Step 1: Running SOLID, Interface-First & 350-Line Limit Audit..."
bash .agents/skills/solid-code-reviewer/scripts/audit_solid.sh
echo ""

echo "Step 2: Running Enterprise Readiness (Pillars 1-6) Audit..."
bash .agents/skills/enterprise-readiness-auditor/scripts/audit_enterprise.sh
echo ""

echo "Step 3: Running MCP Protocol & Skill Governance Audit..."
bash .agents/skills/mcp-protocol-governor/scripts/audit_mcp.sh
echo ""

echo "Step 4: Running Legacy MCP Gateway Enterprise Complaints & Gaps Audit..."
bash .agents/skills/mcp-enterprise-gap-auditor/scripts/audit_mcp_gaps.sh
echo ""

echo "Step 5: Running Zero-Downtime & Dynamic Control Plane Audit..."
bash .agents/skills/zero-downtime-control-plane-auditor/scripts/audit_zero_downtime.sh
echo ""

echo "Step 6: Running Rust Cargo Tests & Type Checks..."
cargo check --all-targets
if command -v cargo-nextest >/dev/null 2>&1; then
  echo "Using cargo-nextest for fast parallel test execution..."
  CI=1 cargo nextest run
else
  cargo test
fi

echo ""
echo "============================================================"
echo " [PASSED] 100% ENTERPRISE GOVERNANCE & ARCHITECTURE VERIFIED"
echo "============================================================"
