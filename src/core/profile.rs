// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

/// Enterprise runtime profile controlling feature opt-outs, GitOps immutability, and memory tuning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseRuntimeProfile {
    /// Whether the embedded Admin Web UI is enabled to serve static assets and dashboard views.
    pub ui_enabled: bool,
    /// Whether GitOps immutability is enforced, disabling dynamic REST API configuration mutations.
    pub immutable_config: bool,
    /// Whether lean memory profile is active, skipping UI event buffers and optimizing memory.
    pub lean_memory: bool,
}

impl Default for EnterpriseRuntimeProfile {
    fn default() -> Self {
        Self {
            ui_enabled: true,
            immutable_config: false,
            lean_memory: false,
        }
    }
}

impl EnterpriseRuntimeProfile {
    /// Detect runtime profile from environment variables and defaults.
    pub fn from_env() -> Self {
        let ui_enabled = std::env::var("PROPYLON_UI_ENABLED")
            .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "no" | "off"))
            .unwrap_or(true);

        let immutable_config = std::env::var("PROPYLON_IMMUTABLE_CONFIG")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);

        let lean_memory = std::env::var("PROPYLON_LEAN_PROFILE")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(!ui_enabled);

        Self {
            ui_enabled,
            immutable_config,
            lean_memory,
        }
    }

    /// Builder method to toggle UI
    pub fn with_ui_enabled(mut self, enabled: bool) -> Self {
        self.ui_enabled = enabled;
        self
    }

    /// Builder method to toggle GitOps immutability
    pub fn with_immutable_config(mut self, immutable: bool) -> Self {
        self.immutable_config = immutable;
        self
    }

    /// Builder method to toggle lean memory
    pub fn with_lean_memory(mut self, lean: bool) -> Self {
        self.lean_memory = lean;
        self
    }

    /// Check if dynamic runtime configuration mutation is permitted.
    pub fn check_mutation_allowed(&self) -> Result<(), &'static str> {
        if self.immutable_config {
            Err("Dynamic runtime configuration mutation via REST API is prohibited under enterprise GitOps immutability policy. Updates must be applied declaratively via propylon.yaml.")
        } else {
            Ok(())
        }
    }
}

/// Helper to read current resident memory usage (RSS) in megabytes.
pub fn get_process_memory_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return (kb / 1024.0 * 100.0).round() / 100.0;
                    }
                }
            }
        }
    }
    0.0
}
