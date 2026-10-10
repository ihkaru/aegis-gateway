#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Enterprise Mock, Stub & Dummy Detection Scanner (Full-Stack: Rust & UI)
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "============================================================"
echo "    AEGIS GATEWAY: FULL-STACK ZERO-MOCK INTEGRITY AUDIT     "
echo "============================================================"

VIOLATIONS=0
GREP_CMD="/usr/bin/grep"

# Dynamic detection of production directories (Rust + Svelte/TS UI)
PROD_DIRS=()
[ -d "src" ] && PROD_DIRS+=("src")
[ -d "ui/src" ] && PROD_DIRS+=("ui/src")
for crate in crates/*/src; do
    [ -d "$crate" ] && PROD_DIRS+=("$crate")
done

echo "Scanning production directories: ${PROD_DIRS[*]}"

# --- RUST PRODUCTION CHECKS ---

# Check 1: Hardcoded mock execution closures in Rust
MOCK_CLOSURES=$($GREP_CMD -rnE '\|\|(\s+)?Ok\(json!\(\{\s*"status":\s*"executed"' src/ || true)
if [ -n "$MOCK_CLOSURES" ]; then
    echo "[FAIL] Detected hardcoded mock execution closures in Rust production path:"
    echo "$MOCK_CLOSURES"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] No hardcoded mock execution closures in Rust."
fi

# Check 2: Rust incomplete execution stubs (todo! / unimplemented!)
STUBS=$($GREP_CMD -rnE '\b(todo!|unimplemented!)\b' src/ || true)
if [ -n "$STUBS" ]; then
    echo "[FAIL] Detected incomplete implementation stubs (todo! / unimplemented!) in src/:"
    echo "$STUBS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] No todo! or unimplemented! stubs found in src/."
fi

# Check 3: Mock struct definitions outside of tests in Rust
MOCK_STRUCTS=$($GREP_CMD -rnE '^\s*(pub\s+)?struct\s+Mock[A-Za-z0-9_]+' src/ || true)
if [ -n "$MOCK_STRUCTS" ]; then
    echo "[FAIL] Detected Mock struct definitions in production source files:"
    echo "$MOCK_STRUCTS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] No production Mock struct declarations found."
fi

# Check 4: Simulated "status": "executed" strings in Rust
SIMULATED_STRINGS=$($GREP_CMD -rn '"status": "executed"' src/ || true)
if [ -n "$SIMULATED_STRINGS" ]; then
    echo "[FAIL] Detected fake 'status': 'executed' response strings in production path:"
    echo "$SIMULATED_STRINGS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] No fake 'status': 'executed' response strings found."
fi

# Check 5: Unsafe code usage in production Rust sources
UNSAFE_BLOCKS=$($GREP_CMD -rnE '\bunsafe\s*\{' src/ || true)
if [ -n "$UNSAFE_BLOCKS" ]; then
    echo "[FAIL] Forbidden unsafe blocks detected in production Rust files:"
    echo "$UNSAFE_BLOCKS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] Zero unsafe blocks in production Rust sources."
fi

# --- UI & FRONTEND PRODUCTION CHECKS (ui/src/) ---
if [ -d "ui/src" ]; then
    # Check 6: Unfinished TODO / FIXME developer stubs in production UI
    UI_STUBS=$($GREP_CMD -rnE '\b(TODO|FIXME)\b' ui/src/ || true)
    if [ -n "$UI_STUBS" ]; then
        echo "[FAIL] Detected unfinished TODO/FIXME stubs in UI production path:"
        echo "$UI_STUBS"
        VIOLATIONS=$((VIOLATIONS + 1))
    else
        echo "[PASS] Zero unfinished TODO/FIXME stubs in ui/src/."
    fi

    # Check 7: Dummy/Mock generators or fake data indicators in UI
    UI_DUMMIES=$($GREP_CMD -rnE '\b(dummyData|fakeData|mockResponse|mockApi)\b' ui/src/ || true)
    if [ -n "$UI_DUMMIES" ]; then
        echo "[FAIL] Detected fake/dummy data generators in UI production files:"
        echo "$UI_DUMMIES"
        VIOLATIONS=$((VIOLATIONS + 1))
    else
        echo "[PASS] Zero fake/dummy data generators in ui/src/."
    fi

    # Check 8: Fake latency simulators in ui/src/
    UI_FAKES=$($GREP_CMD -rnE 'setTimeout\s*\(\s*\(\)\s*=>\s*\{.*status.*200' ui/src/ || true)
    if [ -n "$UI_FAKES" ]; then
        echo "[FAIL] Detected fake network latency simulators in ui/src/:"
        echo "$UI_FAKES"
        VIOLATIONS=$((VIOLATIONS + 1))
    else
        echo "[PASS] Zero fake network simulators in ui/src/."
    fi
fi

echo "------------------------------------------------------------"
if [ "$VIOLATIONS" -gt 0 ]; then
    echo ">>> [AUDIT REJECTED] Found $VIOLATIONS full-stack mock/dummy integrity violation(s)!"
    echo "    Aegis Gateway strictly forbids production mocks across both backend and UI."
    exit 1
else
    echo ">>> [AUDIT PASSED] 100% Full-Stack Production Integrity (Rust & UI Verified)."
    exit 0
fi
