#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo " [AEGIS AUDIT] MCP Protocol & Skill Governance Audit"
echo "============================================================"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
cd "$PROJECT_ROOT"

FAILURES=0

check_item() {
  local title="$1"
  local pattern="$2"
  local target_dir="$3"
  
  if grep -rn "$pattern" "$target_dir" >/dev/null 2>&1; then
    echo "  [PASS] $title"
  else
    echo "  [FAIL] $title (Pattern '$pattern' missing in $target_dir)"
    FAILURES=$((FAILURES + 1))
  fi
}

echo "--> Verifying Progressive Disclosure (L0/L1/L2)..."
check_item "Tier ladder enum defined" "pub enum DisclosureTier" "src/core"
check_item "L0 brief purpose projector" "L0" "src/core"
check_item "L1 signature & required guidance" "L1" "src/core"
check_item "L2 full schema projector" "L2" "src/core"

echo "--> Verifying Anti-Poisoning & Injection Defenses..."
check_item "Tool poisoning scanner defined" "pub trait PoisonScanner" "src/core"
check_item "Suspicious instruction detection" "detect_injection" "src/core"

echo "--> Verifying Centralized Skill Registry..."
check_item "SkillRegistry trait defined" "pub trait SkillRegistry" "src/core"
check_item "Progressive skill document loader" "load_skill" "src/core"

if [ "$FAILURES" -eq 0 ]; then
  echo ">>> [SUCCESS] All MCP Protocol & Skill governance checks passed!"
  exit 0
else
  echo ">>> [ERROR] MCP Protocol governance audit FAILED with $FAILURES violation(s)!"
  exit 1
fi
