# Phase 43: Real In-Situ Analytical Enclave Engine (Zero-Egress Data Processing)

> **Milestone Tag**: `v1.43.0-real_in_situ_analytical_enclave_engine`  
> **Status**: `In Planning & Implementation`  
> **Target Enterprise Gaps & Standards**:
> - Replaces legacy simulated analytical stubs with real filesystem inspection and live Python/SQLite analytical query execution.
> - Enforces Zero-Egress Invariant: Datasets >10MB-100MB are queried directly in server-side enclave memory, returning only structured summaries (<2KB), achieving >99.9% egress bandwidth reduction.
> - Standard Compliance: HIPAA Safe Harbor § 164.514, GDPR Article 32 (Data Minimization), Zero-Mock Architecture Invariant.

---

## 1. Problem Statement & Motivation

In previous iterations, `SandboxedInSituEnclave` used a hardcoded Python script template with simulated numbers (`raw_bytes = 105 * 1024 * 1024`) and virtual paths. This violated the Zero-Mock Invariant:
1. When queried against a real file, it did not execute real column aggregation or SQL parsing.
2. When queried against a missing path, it fabricated simulated size instead of returning honest filesystem errors.
3. It lacked real multi-format parsing (CSV, JSON-Lines, and SQLite databases).

---

## 2. Technical Architecture & Trait Contracts

```
Agent Query (SQL/Aggregation)
       │
       ▼
[ SandboxedInSituEnclave ]
       │
       ├─► 1. Real File Validation (os.path.exists & real st_size)
       ├─► 2. Format Detector (SQLite, CSV, JSON-Lines, Parquet)
       ├─► 3. Server-Side Execution (real sqlite3/csv engine inside hermetic sandbox)
       └─► 4. Strict Zero-Egress Projection: Formats Markdown Summary Table (<2KB)
```

### Key Guarantees:
- **Honest Failures**: Returns `AegisError::NotFound` if the dataset file does not exist on disk.
- **Genuine Egress Reduction**: Calculates real `raw_bytes_read` from `fs::metadata` and real `egress_bytes_returned` from the formatted Markdown table, proving >99.9% reduction.
- **Real Schema Extraction**: Dynamically inspects column headers and row count.
