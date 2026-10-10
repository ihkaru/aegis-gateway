# Phase 32: Direct OTLP Exporter — CNCF OpenTelemetry Protocol (gRPC/HTTP)

## Executive Summary
Phase 32 equips Aegis Gateway with native OpenTelemetry Protocol (OTLP) telemetry export capability directly pushing distributed traces and metrics to any CNCF-compliant collector (Grafana Tempo, Jaeger, Splunk, Datadog, SigNoz, OpenSearch) over gRPC/Protobuf and HTTP. It incorporates a Risk-Adaptive Sampler ensuring 100% capture of security-critical operations (DLP redactions, HITL suspensions, authorization errors) while economically downsampling high-throughput routine tool calls.

---

## 1. Architectural Invariants & Requirements

1. **Native CNCF OTLP v1.x Serialization**:
   - Zero vendor lock-in: direct export to OpenTelemetry Collector endpoints without intermediate log parsing.
   - Support for `HttpJson`, `HttpProtobuf`, and `GrpcProtobuf` wire protocols.
2. **Risk-Adaptive Head/Tail Sampling**:
   - 100% forced sampling for spans involving DLP violations, HITL approvals, or security refusals.
   - Configurable sampling rate (e.g. 10%) for low-risk, high-frequency read-only operations.
3. **Non-Blocking Telemetry Ingestion**:
   - Spans are buffered and dispatched asynchronously to avoid injecting latency into the hot MCP request path.
4. **Interface-First Design**:
   - Contracts `OtlpTelemetryExporter`, `OtlpExportBatch`, and `OtlpExportResult` defined in `src/core/otlp.rs`.
   - Production driver implemented in `src/audit/otlp_exporter.rs`.

---

## 2. Verification Criteria

- [x] OTLP batch serialization generates valid W3C TraceContext trace IDs and span IDs.
- [x] Risk-Adaptive Sampler forces 100% sampling on security events and applies fractional sampling to routine calls.
- [x] Export batch tracks transmission receipts and accurately counts accepted vs dropped spans.
