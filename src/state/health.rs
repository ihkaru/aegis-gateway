// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use crate::core::error::AegisResult;
use crate::core::health::{HealthProbe, HealthReport, ProbeState};
use crate::core::state::DistributedState;

/// Kubernetes Liveness (/healthz) and Readiness (/readyz) Service
pub struct GatewayHealthService {
    state_backend: Arc<dyn DistributedState>,
    service_name: String,
}

impl GatewayHealthService {
    pub fn new(state_backend: Arc<dyn DistributedState>) -> Self {
        Self {
            state_backend,
            service_name: "aegis-gateway".to_string(),
        }
    }

    /// Direct HTTP-compatible status code for /healthz (200 = alive)
    pub async fn healthz(&self) -> (u16, HealthReport) {
        match self.liveness().await {
            Ok(rep) if rep.state == ProbeState::Healthy => (200, rep),
            Ok(rep) => (503, rep),
            Err(e) => (500, HealthReport::fail(&self.service_name, e.to_string())),
        }
    }

    /// Direct HTTP-compatible status code for /readyz (200 = ready to serve traffic)
    pub async fn readyz(&self) -> (u16, HealthReport) {
        match self.readiness().await {
            Ok(rep) if rep.state == ProbeState::Healthy => (200, rep),
            Ok(rep) => (503, rep),
            Err(e) => (500, HealthReport::fail(&self.service_name, e.to_string())),
        }
    }
}

#[async_trait]
impl HealthProbe for GatewayHealthService {
    async fn liveness(&self) -> AegisResult<HealthReport> {
        let mut report = HealthReport::ok(&self.service_name);
        report.details.insert("status".to_string(), "running".to_string());
        report.details.insert("uptime".to_string(), "nominal".to_string());
        Ok(report)
    }

    async fn readiness(&self) -> AegisResult<HealthReport> {
        let probe_key = "__aegis_k8s_readyz_probe__";
        let probe_val = serde_json::json!({"probe": "ok"});

        // Verify distributed state write & read
        let write_res = self.state_backend.set(probe_key, &probe_val, Duration::from_secs(5)).await;
        if let Err(e) = write_res {
            return Ok(HealthReport::fail(&self.service_name, format!("State write failed: {}", e)));
        }

        let read_res = self.state_backend.get(probe_key).await;
        match read_res {
            Ok(Some(_)) => {
                let mut rep = HealthReport::ok(&self.service_name);
                rep.details.insert("distributed_state".to_string(), "accessible".to_string());
                Ok(rep)
            }
            Ok(None) => Ok(HealthReport::fail(&self.service_name, "Probe key missing after write")),
            Err(e) => Ok(HealthReport::fail(&self.service_name, format!("State read failed: {}", e))),
        }
    }
}
