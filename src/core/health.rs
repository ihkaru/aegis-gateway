// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::core::error::AegisResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub state: ProbeState,
    pub component: String,
    pub details: HashMap<String, String>,
    pub timestamp_utc: chrono::DateTime<chrono::Utc>,
}

impl HealthReport {
    pub fn ok(component: impl Into<String>) -> Self {
        Self {
            state: ProbeState::Healthy,
            component: component.into(),
            details: HashMap::new(),
            timestamp_utc: chrono::Utc::now(),
        }
    }

    pub fn fail(component: impl Into<String>, error: impl Into<String>) -> Self {
        let mut details = HashMap::new();
        details.insert("error".to_string(), error.into());
        Self {
            state: ProbeState::Unhealthy,
            component: component.into(),
            details,
            timestamp_utc: chrono::Utc::now(),
        }
    }
}

/// Kubernetes Liveness (/healthz) and Readiness (/readyz) probe abstraction
#[async_trait]
pub trait HealthProbe: Send + Sync {
    /// Liveness check (/healthz): Returns Healthy if gateway process is alive
    async fn liveness(&self) -> AegisResult<HealthReport>;

    /// Readiness check (/readyz): Returns Healthy if downstream dependencies (Redis, state) are reachable
    async fn readiness(&self) -> AegisResult<HealthReport>;
}
