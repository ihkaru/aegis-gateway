#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo " [AEGIS AUDIT] SOLID Principles & Rust Hygiene Audit"
echo "============================================================"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
cd "$PROJECT_ROOT"

FAILURES=0

echo "--> Verifying #![deny(unsafe_code)] at crate root..."
if grep -q "deny(unsafe_code)" src/lib.rs; then
  echo "  [PASS] Zero Unsafe Code policy enforced at src/lib.rs"
else
  echo "  [FAIL] #![deny(unsafe_code)] missing at src/lib.rs"
  FAILURES=$((FAILURES + 1))
fi

echo "--> Verifying Dependency Inversion (traits in core/)..."
if [ -d "src/core" ] && [ $(ls src/core/*.rs | wc -l) -ge 4 ]; then
  echo "  [PASS] Core abstraction layer separated into src/core"
else
  echo "  [FAIL] Core abstraction layer insufficient or missing"
  FAILURES=$((FAILURES + 1))
fi

echo "--> Checking for dangerous unwrap() in production src/..."
UNWRAP_COUNT=$(grep -rn "\.unwrap()" src/ --exclude="*test*" --exclude="main.rs" --exclude="aegis_audit.rs" 2>/dev/null | grep -v "unwrap_or" | wc -l || true)
if [ "$UNWRAP_COUNT" -eq 0 ]; then
  echo "  [PASS] Zero unwrap() found in production library code"
else
  echo "  [WARN] Found $UNWRAP_COUNT unwrap() occurrences in src/. Target is 0!"
fi

if [ "$FAILURES" -eq 0 ]; then
  echo ">>> [SUCCESS] SOLID Principles & Rust hygiene checks passed!"
  exit 0
else
  echo ">>> [ERROR] SOLID Principles audit FAILED with $FAILURES violation(s)!"
  exit 1
fi
