use aegis_gateway::audit::otlp_exporter::NativeOtlpExporter;
use aegis_gateway::core::otlp::{
    OtlpExportBatch, OtlpExportProtocol, OtlpSpanData, OtlpTelemetryExporter,
};
use std::collections::HashMap;

#[test]
fn test_otlp_batch_export_and_json_serialization() {
    let exporter = NativeOtlpExporter::new(
        "http://otel-collector.observability.svc:4318/v1/traces",
        OtlpExportProtocol::HttpJson,
        "aegis-gateway-prod",
        0.1,
    );

    let mut attrs = HashMap::new();
    attrs.insert("mcp.tool".to_string(), "query_duckdb".to_string());
    attrs.insert("mcp.method".to_string(), "tools/call".to_string());
    attrs.insert("http.status_code".to_string(), "200".to_string());

    let span = OtlpSpanData {
        trace_id: "4bf92f3577b34da6a3ce929d0e0e4736".to_string(),
        span_id: "00f067aa0ba902b7".to_string(),
        parent_span_id: None,
        name: "MCP tool execution: query_duckdb".to_string(),
        start_time_unix_nano: 1773129600000000000,
        end_time_unix_nano: 1773129600005000000,
        attributes: attrs,
        status_code: "STATUS_CODE_OK".to_string(),
        is_sampled: true,
    };

    let batch = OtlpExportBatch {
        service_name: "aegis-gateway-prod".to_string(),
        spans: vec![span],
        batch_id: "batch-789123".to_string(),
    };

    // Serialize and inspect standard OTel fields
    let json_output = NativeOtlpExporter::serialize_to_otlp_json(&batch);
    assert!(json_output.contains("resourceSpans"));
    assert!(json_output.contains("4bf92f3577b34da6a3ce929d0e0e4736"));
    assert!(json_output.contains("aegis-gateway-prod"));

    // Dispatch batch
    let res = exporter.export_batch(batch).expect("export batch failed");
    assert_eq!(res.exported_spans_count, 1);
    assert_eq!(res.dropped_spans_count, 0);
    assert_eq!(res.protocol, OtlpExportProtocol::HttpJson);
    assert_eq!(exporter.get_dispatched_batch_count(), 1);
}

#[test]
fn test_risk_adaptive_sampling_policy() {
    let exporter = NativeOtlpExporter::new(
        "http://tempo.observability.svc:4317",
        OtlpExportProtocol::GrpcProtobuf,
        "aegis-gateway-strict",
        0.0, // 0% standard sampling
    );

    // 1. High risk: DLP violation detected -> MUST SAMPLE
    assert!(exporter.should_sample(true, false, false));

    // 2. High risk: HITL suspension triggered -> MUST SAMPLE
    assert!(exporter.should_sample(false, true, false));

    // 3. Security error: Unauthorized / refusal -> MUST SAMPLE
    assert!(exporter.should_sample(false, false, true));

    // 4. Low risk routine read-only tool -> NOT SAMPLED (dropped to save egress cost)
    assert!(!exporter.should_sample(false, false, false));
}

#[test]
fn test_dropped_vs_exported_accounting() {
    let exporter = NativeOtlpExporter::new(
        "http://datadog-agent:4318",
        OtlpExportProtocol::HttpProtobuf,
        "aegis-gateway",
        0.5,
    );

    let sampled_span = OtlpSpanData {
        trace_id: "trace-1".to_string(),
        span_id: "span-1".to_string(),
        parent_span_id: None,
        name: "high-risk-tool".to_string(),
        start_time_unix_nano: 100,
        end_time_unix_nano: 200,
        attributes: HashMap::new(),
        status_code: "OK".to_string(),
        is_sampled: true,
    };

    let unsampled_span = OtlpSpanData {
        trace_id: "trace-2".to_string(),
        span_id: "span-2".to_string(),
        parent_span_id: None,
        name: "routine-ping".to_string(),
        start_time_unix_nano: 300,
        end_time_unix_nano: 400,
        attributes: HashMap::new(),
        status_code: "OK".to_string(),
        is_sampled: false,
    };

    let batch = OtlpExportBatch {
        service_name: "aegis-gateway".to_string(),
        spans: vec![sampled_span, unsampled_span],
        batch_id: "batch-mixed".to_string(),
    };

    let res = exporter.export_batch(batch).unwrap();
    assert_eq!(res.exported_spans_count, 1);
    assert_eq!(res.dropped_spans_count, 1);
}
