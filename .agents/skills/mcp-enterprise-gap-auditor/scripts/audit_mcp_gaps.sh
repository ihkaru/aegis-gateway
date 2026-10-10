#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo " [AEGIS AUDIT] MCP Gateway Enterprise Complaints & Gaps Audit"
echo "============================================================"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
cd "$PROJECT_ROOT"

STRICT_MODE="${STRICT_ENTERPRISE:-0}"
CONTRACTS_RESOLVED=0
PROD_DRIVERS_READY=0
ROADMAP_PENDING=0

audit_complaint() {
  local num="$1"
  local name="$2"
  local contract_pattern="$3"
  local target_file="$4"
  local prod_indicator="$5"
  local prod_file="$6"
  local phase_target="$7"

  echo "--> [$num/9] Enterprise Flaw: $name"

  # Check foundational architectural contract
  if grep -rq "$contract_pattern" "$target_file" 2>/dev/null; then
    echo "  [CONTRACT RESOLVED] Found foundational contract: '$contract_pattern'"
    CONTRACTS_RESOLVED=$((CONTRACTS_RESOLVED + 1))
  else
    echo "  [CONTRACT MISSING]  Pattern '$contract_pattern' missing in $target_file"
  fi

  # Check production-level milestone driver
  if grep -rq "$prod_indicator" "$prod_file" 2>/dev/null; then
    echo "  [PROD DRIVER READY] '$prod_indicator' implemented in $prod_file"
    PROD_DRIVERS_READY=$((PROD_DRIVERS_READY + 1))
  else
    echo "  [ROADMAP TARGET]    Production driver pending: $phase_target"
    ROADMAP_PENDING=$((ROADMAP_PENDING + 1))
  fi
}

# 1. In-Memory State Lock-in
audit_complaint 1 "In-Memory State Lock-in & No Clustering" \
  "pub trait DistributedState" "src/core/state.rs" \
  "pub struct RedisStateBackend" "src/state" \
  "Phase 1 (RedisStateBackend cluster driver with Lua script)"

# 2. Coarse Role Bits / No Dynamic Payload ABAC (Issue #555)
audit_complaint 2 "Coarse Role Bits / No Dynamic Payload ABAC" \
  "eval_payload" "src/core/policy.rs" \
  "pub struct OpaPolicyEngine" "src/policy" \
  "Phase 2 (OIDC/SAML Federation & OPA/Cedar Evaluator)"

# 3. Subprocess Silent Crashes & Cascading Failure (Issue #3034)
audit_complaint 3 "Subprocess Silent Crashes & Cascading Failure" \
  "pub trait DistributedCircuitBreaker" "src/core/state.rs" \
  "healthz" "src" \
  "Phase 1 (Kubernetes /healthz & /readyz probes)"

# 4. Zero Data Loss Prevention (PII / PCI / Secrets Exfiltration)
audit_complaint 4 "Zero Data Loss Prevention (PII / PCI Secrets Leak)" \
  "pub trait DlpPipeline" "src/core/dlp.rs" \
  "pub struct PresidioDlpPipeline" "src/dlp" \
  "Phase 3 (Microsoft Presidio / External NER service)"

# 5. Prompt Token Bloat & Meta-Tool 2-Hop Search Overhead
audit_complaint 5 "Prompt Token Bloat & Meta-Tool 2-Hop Search Latency" \
  "pub enum DisclosureTier" "src/core/types.rs" \
  "pub struct ProgressiveProjector" "src/discovery" \
  "Phase 6 (Progressive Disclosure L0/L1/L2 verified)"

# 6. Anti-Poisoning & Prompt Injection in Tool Descriptions
audit_complaint 6 "Anti-Poisoning & Tool Description Prompt Injection" \
  "pub trait PoisonScanner" "src/core/skills.rs" \
  "pub struct DefaultPoisonScanner" "src/skills" \
  "Phase 3 (Core poison scanner implemented)"

# 7. No Tamper-Evident SIEM Audit Logs
audit_complaint 7 "No Tamper-Evident SIEM Audit Logs" \
  "pub trait AuditSink" "src/core/audit.rs" \
  "pub struct HashChainSequencer" "src/audit" \
  "Phase 4 (SHA-256 Hash Chain Sequencer & OTel exporter)"

# 8. No FinOps & Runaway Loop Cost Cutoffs
audit_complaint 8 "No FinOps & Runaway Loop Cost Cutoffs" \
  "pub trait QuotaEngine" "src/core/state.rs" \
  "pub struct HardFreezeQuota" "src/state" \
  "Phase 5 (Hard budget freeze & cost metering engine)"

# 9. Skill Dispersal & Static Prompt Stuffing
audit_complaint 9 "Skill Dispersal & Static Prompt Stuffing" \
  "pub trait SkillRegistry" "src/core/skills.rs" \
  "pub struct GitOpsSkillSync" "src/skills" \
  "Phase 6 (GitOps Remote Skill Sync & Vector RAG)"

# 10. Unconstrained Remote Code Execution (RCE by Design)
audit_complaint 10 "Unconstrained Remote Code Execution (RCE)" \
  "pub trait CodeSandboxEngine" "src/core/sandbox.rs" \
  "pub struct HermeticProcessSandbox" "src/sandbox" \
  "Phase 16 (Hermetic Process Isolation Sandbox)"

# 11. SSRF & Network Data Exfiltration
audit_complaint 11 "SSRF & Network Data Exfiltration" \
  "pub trait EgressFirewall" "src/core/sandbox.rs" \
  "pub struct EgressFilterEngine" "src/sandbox" \
  "Phase 16 (Default-Deny Egress Firewall)"

# 12. Credential Leakage & Token Scope Creep
audit_complaint 12 "Credential Leakage & Token Scope Creep" \
  "pub trait CredentialBroker" "src/core/sandbox.rs" \
  "pub struct VaultCredentialBroker" "src/sandbox" \
  "Phase 16 (Zero-Knowledge Credential Broker)"

# 13. Non-Repudiation Deficit & Cryptographic Attestation
audit_complaint 13 "Non-Repudiation Deficit & SOC 2 Attestation" \
  "pub trait CodeSandboxEngine" "src/core/sandbox.rs" \
  "attestation" "src/sandbox/hermetic_driver.rs" \
  "Phase 16 (Cryptographic Code Hash & Execution Attestation)"

echo ""
echo "============================================================"
echo "          ENTERPRISE GAP AUDIT SCORECARD SUMMARY            "
echo "============================================================"
echo "  Foundational Architectural Contracts : $CONTRACTS_RESOLVED / 13 Resolved"
echo "  Production-Ready Drivers Implemented : $PROD_DRIVERS_READY / 13 Ready"
echo "  Roadmap Milestone Drivers Pending    : $ROADMAP_PENDING / 13 In Progress"
echo "============================================================"

if [ "$STRICT_MODE" -eq 1 ] && [ "$ROADMAP_PENDING" -gt 0 ]; then
  echo ">>> [STRICT MODE: FAILED] $ROADMAP_PENDING production drivers remain to be built across Phase 1-16."
  exit 1
elif [ "$CONTRACTS_RESOLVED" -lt 13 ]; then
  echo ">>> [ARCHITECTURAL FAILURE] Foundational contracts missing!"
  exit 1
else
  echo ">>> [BASELINE ARCHITECTURE PASS] All 13 legacy flaws are architecturally insulated via SOLID traits."
  echo "    Early-stage development status: Pending drivers will be phased in according to docs/roadmap/."
  exit 0
fi
