#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Enterprise Mock Detection & Production Integrity Scanner
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "============================================================"
echo "    AEGIS GATEWAY: PRODUCTION ZERO-MOCK INTEGRITY AUDIT     "
echo "============================================================"

VIOLATIONS=0

GREP_CMD="/usr/bin/grep"

# Pattern 1: Hardcoded mock execution closures
MOCK_CLOSURES=$($GREP_CMD -rnE '\|\|(\s+)?Ok\(json!\(\{\s*"status":\s*"executed"' src/ || true)
if [ -n "$MOCK_CLOSURES" ]; then
    echo "[FAIL] Detected hardcoded mock execution closures in production path:"
    echo "$MOCK_CLOSURES"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] No hardcoded mock execution closures found."
fi

# Pattern 2: Rust incomplete execution stubs (todo! / unimplemented!) in src/
STUBS=$($GREP_CMD -rnE '\b(todo!|unimplemented!)\b' src/ || true)
if [ -n "$STUBS" ]; then
    echo "[FAIL] Detected incomplete implementation stubs (todo! / unimplemented!) in src/:"
    echo "$STUBS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] No todo! or unimplemented! stubs found in src/."
fi

# Pattern 3: Mock struct definitions outside of #[cfg(test)]
MOCK_STRUCTS=$($GREP_CMD -rnE '^\s*(pub\s+)?struct\s+Mock[A-Za-z0-9_]+' src/ || true)
if [ -n "$MOCK_STRUCTS" ]; then
    echo "[FAIL] Detected Mock struct definitions in production source files:"
    echo "$MOCK_STRUCTS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] No production Mock struct declarations found."
fi

# Pattern 4: Simulated "status": "executed" strings in src/
SIMULATED_STRINGS=$($GREP_CMD -rn '"status": "executed"' src/ || true)
if [ -n "$SIMULATED_STRINGS" ]; then
    echo "[FAIL] Detected fake 'status': 'executed' response strings in production path:"
    echo "$SIMULATED_STRINGS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] No fake 'status': 'executed' response strings found."
fi

echo "------------------------------------------------------------"
if [ "$VIOLATIONS" -gt 0 ]; then
    echo ">>> [AUDIT REJECTED] Found $VIOLATIONS mock integrity violation(s) in src/!"
    echo "    Aegis Gateway strictly forbids production mocks on operational paths."
    exit 1
else
    echo ">>> [AUDIT PASSED] 100% Production-Grade Integrity (Zero Mocks in src/)."
    exit 0
fi
