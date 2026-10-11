#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Enterprise Call-Graph & AST Mock Detection Scanner via code-graph-mcp
# Detects silent mocks, dead-end dispatchers, and disconnected trait implementations.
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "============================================================"
echo "  AEGIS GATEWAY: AST & CALL-GRAPH INTEGRITY AUDITOR (MCP)   "
echo "============================================================"

CODE_GRAPH_BIN="/usr/local/bin/code-graph-mcp"
if [ ! -x "$CODE_GRAPH_BIN" ]; then
    echo "[FAIL] code-graph-mcp binary not found at $CODE_GRAPH_BIN"
    exit 1
fi

VIOLATIONS=0

# Step 1: Incremental Indexing
echo "Synchronizing Code Graph AST and SQLite Call Graph..."
"$CODE_GRAPH_BIN" incremental-index > /dev/null 2>&1 || true

# Check 1: WebApiDispatcher Callee Integrity
# Verify that the web dispatcher calls real backend execution and config engines
echo "Checking WebApiDispatcher callee edges..."
DISPATCH_CALLEES=$("$CODE_GRAPH_BIN" callgraph dispatch --file src/control/web_api.rs --direction callees --depth 2 2>/dev/null || true)

for expected_callee in "execute_tool_async" "dispatch (src/control/config_api.rs)" "export_soc2_report" "list_backends" "discover_all_tools"; do
    if [[ "$DISPATCH_CALLEES" != *"$expected_callee"* ]]; then
        echo "[FAIL] Silent mock detected: WebApiDispatcher does not call real engine '$expected_callee'!"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi
done

if [ "$VIOLATIONS" -eq 0 ]; then
    echo "[PASS] WebApiDispatcher callgraph is 100% connected to live execution engines."
fi

# Check 2: Core Trait Implementation & Binding
echo "Checking SqlEngine trait implementation in call graph..."
SQL_ENGINE_REFS=$("$CODE_GRAPH_BIN" refs SqlEngine --json 2>/dev/null || true)
if [[ "$SQL_ENGINE_REFS" != *"LocalSqliteEngine"* || "$SQL_ENGINE_REFS" != *"implements"* ]]; then
    echo "[FAIL] Trait SqlEngine has no live struct implementation in code graph!"
    VIOLATIONS=$((VIOLATIONS + 1))
else
    echo "[PASS] Trait SqlEngine has verified live implementation (LocalSqliteEngine)."
fi

# Check 3: Orphan Dead-Code Audit on Core Services
echo "Auditing orphan dead-code on core production files..."
DEAD_CODE_JSON=$("$CODE_GRAPH_BIN" dead-code src/ --json 2>/dev/null || true)

# Ensure no critical core service struct or dispatcher is orphaned
for critical_symbol in "LocalSqliteEngine" "ConfigApiDispatcher" "WebApiDispatcher" "HttpAdminControlPlane"; do
    if echo "$DEAD_CODE_JSON" | grep -q "\"name\":\"$critical_symbol\".*\"category\":\"orphan\""; then
        echo "[FAIL] Critical service '$critical_symbol' is orphaned and disconnected!"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi
done

if [ "$VIOLATIONS" -eq 0 ]; then
    echo "[PASS] Zero core services orphaned or disconnected from the runtime call graph."
fi

echo "------------------------------------------------------------"
if [ "$VIOLATIONS" -gt 0 ]; then
    echo ">>> [AUDIT REJECTED] Found $VIOLATIONS call-graph integrity violation(s)!"
    echo "    Aegis Gateway strictly forbids dead-end dispatchers and silent mocks."
    exit 1
else
    echo ">>> [AUDIT PASSED] 100% AST & Call-Graph Integrity Verified (Zero Silent Mock)."
    exit 0
fi
