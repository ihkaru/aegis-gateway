// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::audit::{AuditEvent, AuditSink};
use crate::core::error::AegisResult;

/// Splunk HTTP Event Collector (HEC) Audit Sink
pub struct SplunkHecSink {
    hec_endpoint: String,
    hec_token: String,
    emitted_events: Arc<RwLock<Vec<serde_json::Value>>>,
}

impl SplunkHecSink {
    pub fn new(endpoint: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            hec_endpoint: endpoint.into(),
            hec_token: token.into(),
            emitted_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.hec_endpoint
    }

    pub fn token(&self) -> &str {
        &self.hec_token
    }
}

#[async_trait]
impl AuditSink for SplunkHecSink {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()> {
        let splunk_payload = serde_json::json!({
            "time": event.timestamp.timestamp(),
            "host": "aegis-gateway",
            "source": "mcp-control-plane",
            "sourcetype": "_json",
            "event": event,
        });

        let mut events = self.emitted_events.write().await;
        events.push(splunk_payload);
        Ok(())
    }
}

/// Datadog Logs API Audit Sink
pub struct DatadogAuditSink {
    api_endpoint: String,
    emitted_events: Arc<RwLock<Vec<serde_json::Value>>>,
}

impl DatadogAuditSink {
    pub fn new(api_endpoint: impl Into<String>) -> Self {
        Self {
            api_endpoint: api_endpoint.into(),
            emitted_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.api_endpoint
    }
}


#[async_trait]
impl AuditSink for DatadogAuditSink {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()> {
        let dd_payload = serde_json::json!({
            "ddsource": "aegis-gateway",
            "service": "mcp-proxy",
            "message": serde_json::to_string(event).unwrap_or_default(),
            "status": "info",
            "tags": format!("tenant:{},user:{}", event.caller.tenant_id.as_str(), event.caller.subject),
        });

        let mut events = self.emitted_events.write().await;
        events.push(dd_payload);
        Ok(())
    }
}

/// Multiplexed audit fan-out sink forwarding events concurrently to multiple sinks
pub struct MultiplexedAuditSink {
    sinks: Vec<Arc<dyn AuditSink>>,
}

impl MultiplexedAuditSink {
    pub fn new(sinks: Vec<Arc<dyn AuditSink>>) -> Self {
        Self { sinks }
    }

    pub fn with_sink(mut self, sink: Arc<dyn AuditSink>) -> Self {
        self.sinks.push(sink);
        self
    }
}

#[async_trait]
impl AuditSink for MultiplexedAuditSink {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()> {
        for sink in &self.sinks {
            sink.emit(event).await?;
        }
        Ok(())
    }
}
