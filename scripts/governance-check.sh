#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

echo "============================================================"
echo "    AEGIS GATEWAY: UNIFIED ENTERPRISE GOVERNANCE RUNNER     "
echo "============================================================"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "Step 1: Running Full-Stack Zero-Mock & File Limit Auditor..."
bash scripts/audit_mock_detection.sh
echo ""

echo "Step 1B: Running Call-Graph & AST Integrity Auditor (code-graph-mcp)..."
bash scripts/audit_callgraph_integrity.sh
echo ""

echo "Step 2: Running Rust Type Checks & Target Validation..."
cargo check --all-targets
echo ""

echo "Step 3: Running Unified Parallel Test & Governance Battery..."
if command -v cargo-nextest >/dev/null 2>&1; then
  echo "Using cargo-nextest for ultra-fast work-stealing parallel execution..."
  cargo test --no-run -j 2
  CI=1 cargo nextest run -j 2
else
  echo "Running standard cargo test runner..."
  cargo test -j 2
fi

echo ""
echo "============================================================"
echo " [PASSED] 100% ENTERPRISE GOVERNANCE & ARCHITECTURE VERIFIED"
echo "============================================================"
