use crate::core::error::AegisError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported OpenTelemetry wire export protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OtlpExportProtocol {
    HttpJson,
    HttpProtobuf,
    GrpcProtobuf,
}

/// Normalized OpenTelemetry Span data structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtlpSpanData {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_time_unix_nano: u64,
    pub end_time_unix_nano: u64,
    pub attributes: HashMap<String, String>,
    pub status_code: String,
    pub is_sampled: bool,
}

/// A batch of spans prepared for dispatch to an OTLP collector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtlpExportBatch {
    pub service_name: String,
    pub spans: Vec<OtlpSpanData>,
    pub batch_id: String,
}

/// Receipt confirming telemetry batch transmission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtlpExportResult {
    pub exported_spans_count: usize,
    pub dropped_spans_count: usize,
    pub endpoint: String,
    pub protocol: OtlpExportProtocol,
}

/// Interface contract for OpenTelemetry (OTLP) telemetry export.
pub trait OtlpTelemetryExporter: Send + Sync {
    /// Export a batch of distributed trace spans to the configured OTLP collector.
    fn export_batch(&self, batch: OtlpExportBatch) -> Result<OtlpExportResult, AegisError>;

    /// Determine if an operation must be sampled based on security risk factors.
    fn should_sample(&self, has_dlp_alert: bool, has_hitl_suspension: bool, is_error: bool) -> bool;
}
