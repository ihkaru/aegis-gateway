#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo "    AEGIS: ZERO-DOWNTIME & DYNAMIC CONTROL PLANE AUDIT     "
echo "============================================================"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
cd "$PROJECT_ROOT"

echo "Check 1: Verifying Interface-First Control Plane & Cluster Traits..."
for f in "src/core/control_plane.rs" "src/core/cluster_sync.rs"; do
  if [ ! -f "$f" ]; then
    echo " [FAIL] Missing required core trait file: $f"
    exit 1
  fi
done

grep -q "pub trait DynamicControlPlane" src/core/control_plane.rs || {
  echo " [FAIL] DynamicControlPlane trait missing from src/core/control_plane.rs"
  exit 1
}

grep -q "pub trait ClusterSyncEngine" src/core/cluster_sync.rs || {
  echo " [FAIL] ClusterSyncEngine trait missing from src/core/cluster_sync.rs"
  exit 1
}
echo " [OK] Core traits DynamicControlPlane and ClusterSyncEngine verified."

echo "Check 2: Verifying Production Drivers & Atomic State Swappers..."
for f in "src/control/mod.rs" "src/cluster/mod.rs"; do
  if [ ! -f "$f" ]; then
    echo " [FAIL] Missing required driver file: $f"
    exit 1
  fi
done

grep -q "HttpAdminControlPlane" src/control/mod.rs || {
  echo " [FAIL] HttpAdminControlPlane missing in src/control/mod.rs"
  exit 1
}

grep -q "DistributedPubSubClusterSync" src/cluster/mod.rs || {
  echo " [FAIL] DistributedPubSubClusterSync missing in src/cluster/mod.rs"
  exit 1
}
echo " [OK] Production runtime drivers verified."

echo "Check 3: Scanning for Anti-Patterns (No process::exit in runtime handlers)..."
if grep -rn "process::exit" src/control/ src/cluster/; then
  echo " [FAIL] Forbidden process::exit detected in dynamic control plane paths!"
  exit 1
fi
echo " [OK] Zero termination traps detected in dynamic runtime paths."

echo "Check 4: Verifying 350-Line Limit on Control Plane files..."
for f in src/core/control_plane.rs src/core/cluster_sync.rs src/control/mod.rs src/cluster/mod.rs; do
  lines=$(wc -l < "$f")
  if [ "$lines" -gt 350 ]; then
    echo " [FAIL] File $f exceeds 350 lines (got $lines)"
    exit 1
  fi
done
echo " [OK] All control plane and cluster files strictly <= 350 lines."

echo "Check 5: Running Empirical Zero-Downtime Hot-Reload Regression Tests..."
if command -v cargo-nextest >/dev/null 2>&1; then
  CI=1 cargo nextest run --test phase27_dynamic_control_plane_and_hot_reload_test --test phase28_cluster_sync_engine_test
else
  cargo test --test phase27_dynamic_control_plane_and_hot_reload_test --test phase28_cluster_sync_engine_test --quiet
fi

echo "============================================================"
echo " [PASSED] ZERO-DOWNTIME & DYNAMIC CONTROL PLANE AUDIT PASSED"
echo "============================================================"
