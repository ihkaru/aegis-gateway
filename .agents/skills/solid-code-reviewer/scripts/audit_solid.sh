#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo " [AEGIS AUDIT] SOLID, Interface-First & 350-Line Limit Audit"
echo "============================================================"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
cd "$PROJECT_ROOT"

FAILURES=0

echo "--> 1. Verifying Maximum File Length (Limit: <= 350 lines per file)..."
SCAN_DIRS="src tests scripts .agents ui/src docs"
OVERSIZED=$(find $SCAN_DIRS -type d \( -name "node_modules" -o -name "dist" -o -name "target" -o -name ".git" \) -prune -o -type f \( -name "*.rs" -o -name "*.sh" -o -name "*.md" -o -name "*.toml" -o -name "*.svelte" -o -name "*.ts" \) -exec wc -l {} + | awk '$1 > 350 && $2 != "total" { print "  [FAIL] File " $2 " exceeds limit: " $1 " lines (Max: 350)!"; failed++ } END { exit (failed > 0 ? 1 : 0) }' || true)

if [ -n "$OVERSIZED" ]; then
  echo "$OVERSIZED"
  FAILURES=$((FAILURES + 1))
else
  MAX_FILE=$(find $SCAN_DIRS -type d \( -name "node_modules" -o -name "dist" -o -name "target" -o -name ".git" \) -prune -o -type f \( -name "*.rs" -o -name "*.sh" -o -name "*.md" -o -name "*.toml" -o -name "*.svelte" -o -name "*.ts" \) -exec wc -l {} + | grep -v " total$" | sort -n | tail -n 1)
  echo "  [PASS] All files within <= 350 lines limit (Largest: $MAX_FILE)"
fi

echo "--> 2. Verifying Zero Unsafe Code policy (#![deny(unsafe_code)])..."
if grep -q "deny(unsafe_code)" src/lib.rs && grep -q "deny(unsafe_code)" src/main.rs; then
  echo "  [PASS] Zero Unsafe Code policy enforced at src/lib.rs and src/main.rs"
else
  echo "  [FAIL] #![deny(unsafe_code)] missing at crate roots"
  FAILURES=$((FAILURES + 1))
fi

echo "--> 3. Verifying Interface-First Principle (Core traits in src/core)..."
CORE_TRAITS=("DistributedState" "PolicyEngine" "DlpPipeline" "AuditSink" "SkillRegistry" "QuotaEngine" "CodeSandboxEngine")
for trait_name in "${CORE_TRAITS[@]}"; do
  if grep -rq "pub trait $trait_name" src/core/; then
    echo "  [PASS] Trait contract '$trait_name' defined in src/core"
  else
    echo "  [FAIL] Missing trait contract '$trait_name' in src/core"
    FAILURES=$((FAILURES + 1))
  fi
done

echo "--> 4. Verifying Dependency Injection (DI) Pattern in AegisGateway..."
if grep -q "pub struct AegisGateway" src/lib.rs && \
   grep -q "Arc<dyn DistributedState>" src/lib.rs && \
   grep -q "Arc<dyn PolicyEngine>" src/lib.rs && \
   grep -q "Arc<dyn DlpPipeline>" src/lib.rs && \
   grep -q "Arc<dyn AuditSink>" src/lib.rs && \
   grep -q "Arc<dyn SkillRegistry>" src/lib.rs; then
  echo "  [PASS] AegisGateway constructor uses strict trait Dependency Injection"
else
  echo "  [FAIL] AegisGateway violates Dependency Injection; concrete types coupled"
  FAILURES=$((FAILURES + 1))
fi

echo "--> 5. Checking for dangerous bare unwrap() in production src/..."
UNWRAP_COUNT=$(grep -rn "\.unwrap()" src/ --exclude="*test*" --exclude="main.rs" --exclude="aegis_audit.rs" 2>/dev/null | grep -v "unwrap_or" | wc -l || true)
if [ "$UNWRAP_COUNT" -eq 0 ]; then
  echo "  [PASS] Zero unwrap() found in production library code"
else
  echo "  [FAIL] Found $UNWRAP_COUNT unwrap() occurrences in src/. Target is 0!"
  FAILURES=$((FAILURES + 1))
fi

if [ "$FAILURES" -eq 0 ]; then
  echo ">>> [SUCCESS] SOLID, Interface-First & 350-line checks PASSED!"
  exit 0
else
  echo ">>> [ERROR] SOLID architecture audit FAILED with $FAILURES violation(s)!"
  exit 1
fi
