# Phase 3: Real-Time DLP & AI Safety Guardrails

> **Milestone Tag**: `v0.4.0-dlp-guardrails`  
> **Status**: `In Progress` (Regex & PII redaction pipeline verified; Presidio & AI guardrails planned)

---

## 1. Objectives

Prevent accidental or malicious exfiltration of sensitive organizational data (PII, PCI-DSS, HIPAA PHI, enterprise API keys) during AI tool execution, and defend against adversarial tool poisoning / prompt injection attacks targeting coding agents.

---

## 2. Architecture & Contracts

Defined in [`src/core/dlp.rs`](../../src/core/dlp.rs) and [`src/core/skills.rs`](../../src/core/skills.rs):

```rust
#[async_trait]
pub trait DlpPipeline: Send + Sync {
    async fn sanitize_response(&self, payload: Value) -> AegisResult<(Value, Vec<DlpFinding>)>;
    async fn inspect_request_arguments(&self, arguments: &Value) -> AegisResult<Vec<DlpFinding>>;
}

pub trait PoisonScanner: Send + Sync {
    fn detect_injection(&self, text: &str) -> AegisResult<()>;
}
```

### Defense Layers

1. **Request Argument Inspection**:
   - Analyzes tool parameters *before* execution to prevent data exfiltration attempts.
   - Detects leakage of internal credentials, AWS session tokens, or private certificates passed as arguments.
2. **Response Payload Sanitization**:
   - Intercepts tool execution outputs before they reach the LLM context window.
   - Automatically redacts credit cards (Luhn algorithm), Social Security Numbers, emails, IP addresses, and private tokens.
3. **Tool Description Poisoning Protection (OWASP LLM07)**:
   - Scans dynamic tool metadata and `SKILL.md` bundles for adversarial system prompt injections (e.g., hidden `<system>` tags, instructions telling the agent to ignore previous constraints).

---

## 3. Milestones & Checklist

- [x] **3.1 Core DLP Pipeline Abstraction**: Implement `DlpPipeline`, `SensitivityLevel`, and `DlpFinding` (Empirically verified in `tests/enterprise_governance_test.rs`).
- [x] **3.2 Tool Poisoning Detection**: Implement `DefaultPoisonScanner` detecting override triggers and hidden prompt injections in tool descriptions.
- [x] **3.3 PII Sanitizer Implementation**: Implement `PiiDlpPipeline` providing recursive JSON traversal and regex-based redaction for emails, credit cards, and API secrets.
- [ ] **3.4 External NER / Presidio Integration**: Connect to Microsoft Presidio / HuggingFace DeBERTa microservice for multilingual, context-aware named entity recognition.
- [ ] **3.5 Compliance Profiling**: Support configurable regulatory profiles:
  - `HIPAA`: Medical record number, health insurance IDs, medical terminology masking.
  - `PCI-DSS 4.0`: Strict PAN truncation (first 6, last 4) with Luhn checksum validation.
  - `GDPR`: Personal name, national identification, and biometric data masking.
- [ ] **3.6 Sub-Millisecond Rust SIMD Pipeline**: Benchmark and optimize regex & Boyer-Moore pattern matching to achieve <2ms p99 latency overhead for payload scanning.
