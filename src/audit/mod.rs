// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use tracing::info;

use crate::core::audit::{AuditEvent, AuditSink};
use crate::core::error::AegisResult;

pub mod otel;
pub mod sequencer;
pub mod siem;

pub use otel::OtelAuditSink;
pub use sequencer::HashChainSequencer;
pub use siem::{DatadogAuditSink, MultiplexedAuditSink, SplunkHecSink};

/// High-throughput structured SIEM audit logger
#[derive(Default)]
pub struct StructuredAuditLogger;


impl StructuredAuditLogger {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AuditSink for StructuredAuditLogger {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()> {
        let serialized = serde_json::to_string(event)
            .map_err(|e| crate::core::error::AegisError::AuditError(e.to_string()))?;

        // Structured JSON log output ready for Datadog / Vector / FluentBit daemon ingestion
        info!(target: "aegis::audit", event_json = %serialized, "AUDIT_EVENT_EMITTED");
        Ok(())
    }
}
