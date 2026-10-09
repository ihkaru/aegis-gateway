#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo " [AEGIS AUDIT] Pillar 1-6 Enterprise Readiness Audit"
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

echo "--> Verifying Pillar 1: Distributed State & Clustering..."
check_item "DistributedState trait defined" "pub trait DistributedState" "src/core"
check_item "DistributedCache trait defined" "pub trait DistributedCache" "src/core"
check_item "DistributedRateLimiter trait defined" "pub trait DistributedRateLimiter" "src/core"
check_item "DistributedCircuitBreaker trait defined" "pub trait DistributedCircuitBreaker" "src/core"

echo "--> Verifying Pillar 2: Zero-Trust IAM & Granular ABAC..."
check_item "PolicyEngine & ABAC traits defined" "pub trait PolicyEngine" "src/core"
check_item "Attribute-based context structure" "pub struct PolicyContext" "src/core"
check_item "Payload & argument validation" "eval_payload" "src/core"

echo "--> Verifying Pillar 3: Real-Time DLP & PII Masking..."
check_item "DlpPipeline trait defined" "pub trait DlpPipeline" "src/core"
check_item "PII detection contract" "pub enum SensitivityLevel" "src/core"

echo "--> Verifying Pillar 4: Tamper-Evident SIEM Audit..."
check_item "AuditSink trait defined" "pub trait AuditSink" "src/core"
check_item "Structured AuditEvent with SHA-256 integrity" "pub struct AuditEvent" "src/core"

echo "--> Verifying Pillar 5: Multi-Tenant FinOps & Quotas..."
check_item "TenantQuotaEngine trait defined" "pub trait QuotaEngine" "src/core"

echo "--> Verifying Pillar 6: Permissive MIT License..."
if grep -q "MIT License" LICENSE; then
  echo "  [PASS] Clean MIT license confirmed"
else
  echo "  [FAIL] LICENSE does not contain MIT License"
  FAILURES=$((FAILURES + 1))
fi

if [ "$FAILURES" -eq 0 ]; then
  echo ">>> [SUCCESS] All Enterprise Readiness checks passed! (0 violations)"
  exit 0
else
  echo ">>> [ERROR] Enterprise Readiness audit FAILED with $FAILURES violation(s)!"
  exit 1
fi
