#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo "    AEGIS GATEWAY: UNIFIED ENTERPRISE GOVERNANCE RUNNER     "
echo "============================================================"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "Step 1: Running SOLID Principles & Rust Hygiene Audit..."
bash .agents/skills/solid-code-reviewer/scripts/audit_solid.sh
echo ""

echo "Step 2: Running Enterprise Readiness (Pillars 1-6) Audit..."
bash .agents/skills/enterprise-readiness-auditor/scripts/audit_enterprise.sh
echo ""

echo "Step 3: Running MCP Protocol & Skill Governance Audit..."
bash .agents/skills/mcp-protocol-governor/scripts/audit_mcp.sh
echo ""

echo "Step 4: Running Rust Cargo Tests & Type Checks..."
cargo check --all-targets
cargo test

echo ""
echo "============================================================"
echo " [PASSED] 100% ENTERPRISE GOVERNANCE & ARCHITECTURE VERIFIED"
echo "============================================================"
