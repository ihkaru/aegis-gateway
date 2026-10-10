#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo "    AEGIS GATEWAY: UNIFIED ENTERPRISE GOVERNANCE RUNNER     "
echo "============================================================"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "Step 1: Running Rust Type Checks & Target Validation..."
cargo check --all-targets
echo ""

echo "Step 2: Running Unified Parallel Test & Governance Battery (151 tests)..."
if command -v cargo-nextest >/dev/null 2>&1; then
  echo "Using cargo-nextest for ultra-fast work-stealing parallel execution..."
  cargo test --no-run -j 2
  CI=1 cargo nextest run
else
  echo "Running standard cargo test runner..."
  cargo test -j 2
fi

echo ""
echo "============================================================"
echo " [PASSED] 100% ENTERPRISE GOVERNANCE & ARCHITECTURE VERIFIED"
echo "============================================================"
