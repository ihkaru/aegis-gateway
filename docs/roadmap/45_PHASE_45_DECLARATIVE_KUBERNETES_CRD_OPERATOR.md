# Phase 45: Declarative Kubernetes CRD Operator & Engine

> **Milestone Tag**: `v1.45.0-declarative_kubernetes_crd_operator`  
> **Status**: `In Planning & Implementation`  
> **Target Enterprise Gaps & Standards**:
> - Reconciles GitOps Kubernetes Custom Resources (`PropylonBackend` and `PropylonPolicy`) with the active gateway control plane without restart.
> - Strict Schema Validation: Rejects malformed transport specifications, unsafe command injections, or invalid policy tiers before applying.
> - Standard Compliance: Kubernetes Operator Best Practices, CNCF GitOps Principles.

---

## 1. Problem Statement & Motivation

While `README.md` documents `apiVersion: propylon.enterprise.io/v1alpha1` manifests, enterprise platform engineers require:
1. Native parsing and validation of declarative CRD YAML/JSON specifications.
2. In-memory reconciler converting CRD definitions into live `BackendRegistration` and `PolicyTier` states.
3. Automated drift detection reporting whether the cluster state matches the declared GitOps manifest.

---

## 2. Technical Architecture & Trait Contracts

```rust
pub trait CrdReconciliationEngine: Send + Sync {
    fn validate_manifest(&self, yaml_content: &str) -> PropylonResult<ValidatedCrdManifest>;
    async fn reconcile_manifest(&self, manifest: &ValidatedCrdManifest) -> PropylonResult<ReconciliationReport>;
}
```

### Key Guarantees:
- **Zero-Downtime Application**: Applies backend and policy changes directly to `DynamicControlPlane` via atomic state swapping.
- **Strict Validation**: Validates transport (`stdio`, `sse`, `streamable_http`), endpoints, timeouts, and policy tiers.
- **Honest Feedback**: Returns granular error messages on validation failure.
