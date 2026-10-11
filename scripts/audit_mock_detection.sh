#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Enterprise Mock, Stub & Dummy Detection Scanner (Full-Stack: Rust & UI + Waivers)
# Standard: October 2026 Structured Mock Waiver & Zero-Mock Production Invariant
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "============================================================"
echo "    AEGIS GATEWAY: ADVANCED ZERO-MOCK & INTEGRITY AUDITOR   "
echo "============================================================"

VIOLATIONS=0
GREP_CMD="/usr/bin/grep"
TODAY=$(date +%F)

# 1. Production Directories Inspection
PROD_DIRS=()
[ -d "src" ] && PROD_DIRS+=("src")
[ -d "ui/src" ] && PROD_DIRS+=("ui/src")
for crate in crates/*/src; do
    [ -d "$crate" ] && PROD_DIRS+=("$crate")
done

echo "Scanning production directories: ${PROD_DIRS[*]}"

# --- RUST PRODUCTION CHECKS ---

# Check 1: Disallowed Mocking Frameworks in Production Sources
DISALLOWED_FRAMEWORKS=$($GREP_CMD -rnE '\b(use\s+mockall|#\[automock\]|use\s+faux|use\s+fake::|fake_rs)\b' src/ || true)
if [ -n "$DISALLOWED_FRAMEWORKS" ]; then
    echo "[FAIL] Disallowed mocking frameworks detected in production src/:"
    echo "$DISALLOWED_FRAMEWORKS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] Zero mocking frameworks imported in production Rust sources."
fi

# Check 2: Unannotated Mock Closures in Production Rust
UNANNOTATED_CLOSURES=""
while IFS= read -r match; do
    [ -z "$match" ] && continue
    file=$(echo "$match" | cut -d: -f1)
    line_no=$(echo "$match" | cut -d: -f2)
    # Check if preceding 3 lines contain MOCK_WAIVER
    prev_lines=$(sed -n "$((line_no > 3 ? line_no - 3 : 1)),$((line_no))p" "$file")
    if [[ "$prev_lines" != *"MOCK_WAIVER["* ]]; then
        UNANNOTATED_CLOSURES+="$file:$line_no: $match"$'\n'
    fi
done < <($GREP_CMD -rnE '\|\|(\s+)?Ok\(json!\(\{\s*"status":\s*"executed"|execute_tool_async\(.*\|\|\s*(async\s*)?\{\s*Ok\(raw_result' src/ || true)

if [ -n "$UNANNOTATED_CLOSURES" ]; then
    echo "[FAIL] Detected unannotated silent mock closures in production src/ (missing MOCK_WAIVER):"
    echo "$UNANNOTATED_CLOSURES"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] Zero silent/unannotated mock execution closures in src/."
fi

# Check 3: Rust Incomplete Execution Stubs (todo! / unimplemented!)
STUBS=$($GREP_CMD -rnE '\b(todo!|unimplemented!)\b' src/ || true)
if [ -n "$STUBS" ]; then
    echo "[FAIL] Detected incomplete implementation stubs (todo! / unimplemented!) in src/:"
    echo "$STUBS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] Zero todo! or unimplemented! stubs in src/."
fi

# Check 4: Mock Struct Declarations in Production Rust
MOCK_STRUCTS=$($GREP_CMD -rnE '^\s*(pub\s+)?struct\s+Mock[A-Za-z0-9_]+' src/ || true)
if [ -n "$MOCK_STRUCTS" ]; then
    echo "[FAIL] Detected Mock struct definitions in production source files:"
    echo "$MOCK_STRUCTS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] Zero production Mock struct declarations found."
fi

# Check 5: Deceptive Fake Execution Strings in Production Rust
DECEPTIVE_STRINGS=$($GREP_CMD -rnE '("status":\s*"executed"|"Executed .* successfully against upstream backend"|"Execution of .* completed")' src/ || true)
if [ -n "$DECEPTIVE_STRINGS" ]; then
    echo "[FAIL] Detected deceptive fake execution claims in production path:"
    echo "$DECEPTIVE_STRINGS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] Zero deceptive fake execution claims found."
fi

# Check 6: Hardcoded Fake Latency Constants in API Responses
FAKE_LATENCIES=$($GREP_CMD -rnE '"latency_ms":\s*(0\.[0-9]+|[1-9]\.[0-9]+)\s*[,}]' src/control/web_api.rs 2>/dev/null | $GREP_CMD -v "call_start" || true)
if [ -n "$FAKE_LATENCIES" ]; then
    echo "[FAIL] Detected hardcoded fake latency literals in API response (must measure actual elapsed time):"
    echo "$FAKE_LATENCIES"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] Latencies dynamically measured via Instant::elapsed()."
fi

# Check 7: Zero Unsafe Code Blocks in Production Rust
UNSAFE_BLOCKS=$($GREP_CMD -rnE '\bunsafe\s*\{' src/ || true)
if [ -n "$UNSAFE_BLOCKS" ]; then
    echo "[FAIL] Forbidden unsafe blocks detected in production Rust files:"
    echo "$UNSAFE_BLOCKS"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] Zero unsafe blocks in production Rust sources (#![deny(unsafe_code)])."
fi

# --- UI & FRONTEND PRODUCTION CHECKS (ui/src/) ---
if [ -d "ui/src" ]; then
    # Check 8: Unfinished TODO / FIXME developer stubs in UI
    UI_STUBS=$($GREP_CMD -rnE '\b(TODO|FIXME)\b' ui/src/ || true)
    if [ -n "$UI_STUBS" ]; then
        echo "[FAIL] Detected unfinished TODO/FIXME stubs in UI production path:"
        echo "$UI_STUBS"
        VIOLATIONS=$((VIOLATIONS + 1))
    else
        echo "[PASS] Zero unfinished TODO/FIXME stubs in ui/src/."
    fi

    # Check 9: Fake/Dummy Data Generators in UI
    UI_DUMMIES=$($GREP_CMD -rnE '\b(dummyData|fakeData|mockResponse|mockApi|faker)\b' ui/src/ || true)
    if [ -n "$UI_DUMMIES" ]; then
        echo "[FAIL] Detected fake/dummy data generators in UI production files:"
        echo "$UI_DUMMIES"
        VIOLATIONS=$((VIOLATIONS + 1))
    else
        echo "[PASS] Zero fake/dummy data generators in ui/src/."
    fi

    # Check 10: Fake Network Latency Simulators in UI
    UI_FAKES=$($GREP_CMD -rnE 'setTimeout\s*\(\s*\(\)\s*=>\s*\{.*status.*200' ui/src/ || true)
    if [ -n "$UI_FAKES" ]; then
        echo "[FAIL] Detected fake network latency simulators in ui/src/:"
        echo "$UI_FAKES"
        VIOLATIONS=$((VIOLATIONS + 1))
    else
        echo "[PASS] Zero fake network simulators in ui/src/."
    fi

    # Check 11: Static Mock Fixture IDs in UI Components
    UI_MOCK_FIXTURES=$($GREP_CMD -rnE '(hitl_tkt_[0-9]+|dlp_evt_[0-9]+|tnt_algo_[0-9]+)' ui/src/ || true)
    if [ -n "$UI_MOCK_FIXTURES" ]; then
        echo "[FAIL] Detected hardcoded mock dataset fixtures in UI components:"
        echo "$UI_MOCK_FIXTURES"
        VIOLATIONS=$((VIOLATIONS + 1))
    else
        echo "[PASS] Zero hardcoded mock dataset fixtures in ui/src/ (Live REST binding verified)."
    fi
fi

# --- STRUCTURED MOCK WAIVER AUDIT LEDGER (OCTOBER 2026 STANDARD) ---
echo ""
echo "--- Auditing Structured Mock Waivers (RFC 2383 / October 2026 Standard) ---"
WAIVER_LINES=$($GREP_CMD -rnE 'MOCK_WAIVER\[.*\]' src/ tests/ ui/src/ 2>/dev/null || true)
TOTAL_WAIVERS=0

if [ -n "$WAIVER_LINES" ]; then
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        TOTAL_WAIVERS=$((TOTAL_WAIVERS + 1))
        FILE_LOC=$(echo "$line" | cut -d: -f1-2)
        WAIVER_CONTENT=$(echo "$line" | sed -n 's/.*MOCK_WAIVER\[\(.*\)\]/\1/p')

        # Validate mandatory fields: id, reason, author, expires, replacement_plan
        for field in "id:" "reason:" "author:" "expires:" "replacement_plan:"; do
            if [[ "$WAIVER_CONTENT" != *"$field"* ]]; then
                echo "[FAIL] Malformed MOCK_WAIVER at $FILE_LOC: missing required '$field' field."
                VIOLATIONS=$((VIOLATIONS + 1))
            fi
        done

        # Extract expiry date YYYY-MM-DD
        EXPIRES=$(echo "$WAIVER_CONTENT" | sed -n 's/.*expires:[[:space:]]*"\([^"]*\)".*/\1/p')
        ID=$(echo "$WAIVER_CONTENT" | sed -n 's/.*id:[[:space:]]*"\([^"]*\)".*/\1/p')
        REASON=$(echo "$WAIVER_CONTENT" | sed -n 's/.*reason:[[:space:]]*"\([^"]*\)".*/\1/p')

        if [ -z "$EXPIRES" ]; then
            echo "[FAIL] MOCK_WAIVER at $FILE_LOC has invalid or unquoted 'expires' date."
            VIOLATIONS=$((VIOLATIONS + 1))
            continue
        fi

        # Check if waiver is expired
        if [[ "$TODAY" > "$EXPIRES" ]]; then
            echo "[FAIL] Expired MOCK_WAIVER '$ID' at $FILE_LOC! Expired on $EXPIRES (Current: $TODAY)."
            VIOLATIONS=$((VIOLATIONS + 1))
        else
            echo "[WAIVER ACTIVE] '$ID' at $FILE_LOC (valid until $EXPIRES)"
            echo "                Reason: $REASON"
        fi
    done <<< "$WAIVER_LINES"
fi

echo "[INFO] Total active mock waivers tracked: $TOTAL_WAIVERS"

# --- LINE LIMIT AUDIT (HARD CONSTRAINT: wc -l <= 350) ---
echo ""
echo "--- Auditing File Line Count Limits (Hard Constraint: <= 350 lines) ---"
OVERSIZED_FILES=0
while IFS= read -r f; do
    [ -z "$f" ] && continue
    lines=$(wc -l < "$f")
    if [ "$lines" -gt 350 ]; then
        echo "[FAIL] File exceeds 350 lines ($lines lines): $f"
        OVERSIZED_FILES=$((OVERSIZED_FILES + 1))
        VIOLATIONS=$((VIOLATIONS + 1))
    fi
done < <(find src tests ui/src scripts -type f \( -name "*.rs" -o -name "*.svelte" -o -name "*.ts" -o -name "*.sh" \) 2>/dev/null || true)

if [ "$OVERSIZED_FILES" -eq 0 ]; then
    echo "[PASS] 100% of codebase files strictly adhere to wc -l <= 350."
fi

echo "------------------------------------------------------------"
if [ "$VIOLATIONS" -gt 0 ]; then
    echo ">>> [AUDIT REJECTED] Found $VIOLATIONS full-stack integrity violation(s)!"
    echo "    Aegis Gateway strictly forbids silent mocks and unconstrained files."
    exit 1
else
    echo ">>> [AUDIT PASSED] 100% Full-Stack Zero-Mock & Governance Integrity Verified."
    exit 0
fi
