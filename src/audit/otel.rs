// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::audit::{AuditEvent, AuditSink, OtelTraceContext};
use crate::core::error::AegisResult;

/// OpenTelemetry Distributed Tracing Sink with W3C TraceContext propagation
pub struct OtelAuditSink {
    service_name: String,
    spans: Arc<RwLock<Vec<serde_json::Value>>>,
}

impl OtelAuditSink {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            spans: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Generate W3C compliant TraceContext for request propagation
    pub fn create_trace_context(event_id: &str) -> OtelTraceContext {
        let digest = Sha256::digest(event_id.as_bytes());
        let trace_hash = digest.iter().fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{:02x}", b);
            acc
        });
        let trace_id = trace_hash[..32].to_string();
        let span_id = trace_hash[32..48].to_string();
        let traceparent = format!("00-{}-{}-01", trace_id, span_id);


        OtelTraceContext {
            trace_id,
            span_id,
            traceparent,
        }
    }

    pub async fn recorded_spans(&self) -> Vec<serde_json::Value> {
        self.spans.read().await.clone()
    }
}

impl Default for OtelAuditSink {
    fn default() -> Self {
        Self::new("aegis-gateway")
    }
}

#[async_trait]
impl AuditSink for OtelAuditSink {
    async fn emit(&self, event: &AuditEvent) -> AegisResult<()> {
        let trace_ctx = Self::create_trace_context(&event.event_id);

        let otel_span = serde_json::json!({
            "name": format!("mcp_tool_execution:{}", event.target_resource),
            "trace_id": trace_ctx.trace_id,
            "span_id": trace_ctx.span_id,
            "traceparent": trace_ctx.traceparent,
            "service.name": self.service_name,
            "timestamp_unix_nano": event.timestamp.timestamp_nanos_opt().unwrap_or(0),
            "attributes": {
                "mcp.server": event.target_resource.split(':').next().unwrap_or(""),
                "mcp.tool": event.target_resource.split(':').nth(1).unwrap_or(""),
                "tenant.id": event.caller.tenant_id.as_str(),
                "user.id": event.caller.subject,
                "payload.hash": event.payload_hash_sha256,
            },
            "status": { "code": "OK" }
        });

        let mut spans = self.spans.write().await;
        spans.push(otel_span);
        Ok(())
    }
}
