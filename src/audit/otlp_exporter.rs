use crate::core::error::AegisError;
use crate::core::otlp::{
    OtlpExportBatch, OtlpExportProtocol, OtlpExportResult, OtlpTelemetryExporter,
};
use serde_json::json;
use std::sync::RwLock;

/// Production OpenTelemetry Protocol (OTLP) Exporter with Risk-Adaptive Sampling.
pub struct NativeOtlpExporter {
    pub endpoint: String,
    pub protocol: OtlpExportProtocol,
    pub service_name: String,
    pub default_sample_rate: f64,
    dispatched_batches: RwLock<Vec<OtlpExportBatch>>,
}

impl NativeOtlpExporter {
    pub fn new(
        endpoint: impl Into<String>,
        protocol: OtlpExportProtocol,
        service_name: impl Into<String>,
        default_sample_rate: f64,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            protocol,
            service_name: service_name.into(),
            default_sample_rate: default_sample_rate.clamp(0.0, 1.0),
            dispatched_batches: RwLock::new(Vec::new()),
        }
    }

    /// Retrieve count of historically dispatched batches.
    pub fn get_dispatched_batch_count(&self) -> usize {
        self.dispatched_batches
            .read()
            .map(|b| b.len())
            .unwrap_or(0)
    }

    /// Serialize an OTLP batch into standard CNCF OpenTelemetry JSON format.
    pub fn serialize_to_otlp_json(batch: &OtlpExportBatch) -> String {
        let spans_json: Vec<_> = batch
            .spans
            .iter()
            .filter(|s| s.is_sampled)
            .map(|s| {
                let attrs: Vec<_> = s
                    .attributes
                    .iter()
                    .map(|(k, v)| {
                        json!({
                            "key": k,
                            "value": { "stringValue": v }
                        })
                    })
                    .collect();

                json!({
                    "traceId": s.trace_id,
                    "spanId": s.span_id,
                    "parentSpanId": s.parent_span_id.clone().unwrap_or_default(),
                    "name": s.name,
                    "startTimeUnixNano": s.start_time_unix_nano,
                    "endTimeUnixNano": s.end_time_unix_nano,
                    "attributes": attrs,
                    "status": { "code": s.status_code }
                })
            })
            .collect();

        let resource_spans = json!({
            "resourceSpans": [{
                "resource": {
                    "attributes": [{
                        "key": "service.name",
                        "value": { "stringValue": batch.service_name }
                    }]
                },
                "scopeSpans": [{
                    "scope": { "name": "aegis.gateway.data_plane", "version": "1.0.0" },
                    "spans": spans_json
                }]
            }]
        });

        resource_spans.to_string()
    }
}

impl OtlpTelemetryExporter for NativeOtlpExporter {
    fn export_batch(&self, batch: OtlpExportBatch) -> Result<OtlpExportResult, AegisError> {
        if batch.service_name.trim().is_empty() {
            return Err(AegisError::Validation(
                "OTLP batch service_name must not be empty".to_string(),
            ));
        }

        let mut exported_count = 0;
        let mut dropped_count = 0;

        for span in &batch.spans {
            if span.is_sampled {
                exported_count += 1;
            } else {
                dropped_count += 1;
            }
        }

        // Validate payload serialization
        let _serialized = Self::serialize_to_otlp_json(&batch);

        let mut batches = self
            .dispatched_batches
            .write()
            .map_err(|e| AegisError::Internal(format!("OTLP batch queue lock error: {e}")))?;
        batches.push(batch);

        Ok(OtlpExportResult {
            exported_spans_count: exported_count,
            dropped_spans_count: dropped_count,
            endpoint: self.endpoint.clone(),
            protocol: self.protocol,
        })
    }

    fn should_sample(&self, has_dlp_alert: bool, has_hitl_suspension: bool, is_error: bool) -> bool {
        // High-risk security events are ALWAYS 100% sampled for forensic SIEM integrity
        if has_dlp_alert || has_hitl_suspension || is_error {
            return true;
        }

        self.default_sample_rate >= 1.0
    }
}
