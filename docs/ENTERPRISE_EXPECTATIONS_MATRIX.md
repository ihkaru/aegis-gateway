# Propylon Gateway: Enterprise End-to-End Expectations Matrix

> [!IMPORTANT]
> **Living Contract & Architectural Truth**:
> Dokumen ini memetakan seluruh ekspektasi end-to-end (E2E) para pemangku kepentingan enterprise (Enterprise Architect, CISO, SRE/Platform, Developer) berdasarkan janji spesifikasi di `README.md` dan antarmuka Web UI Control Plane.
> Dokumen ini memisahkan secara transparan antara fitur yang telah terverifikasi empiris (`VERIFIED`), fitur parsial (`PARTIAL`), dan item peta jalan mendatang (`ROADMAP`). Wajib dipatuhi dan diperbarui secara berkala sesuai ketentuan di `/root/GEMINI.md`.

---

## 1. Matriks Ekspektasi Berdasarkan Domain Arsitektur

### A. Data Plane & Wire Protocol (Stateless High-Throughput Ingress)
| No | Ekspektasi Enterprise | Persona Utama | Status | Bukti Verifikasi / Keterangan |
| :--- | :--- | :--- | :--- | :--- |
| A1 | **Dual-Stack Adaptive MCP Negotiation**<br/>Mendukung MCP `2026-07-28` (`Mcp-Method`/`Mcp-Name` headers, anti-desync) dan legacy `2024-11-05` | Developer & Architect | **VERIFIED** | Lulus di `tests/phase7_mcp_wire_test.rs` & `tests/phase9_legacy_parity_test.rs` |
| A2 | **Multi-Transport Stateless Routing**<br/>Dukungan hermetic Stdio, SSE, dan Streamable HTTP tanpa session stickiness | Platform SRE | **VERIFIED** | Lulus di `tests/phase7_mcp_wire_test.rs` & `tests/phase8_backend_multiplexing_test.rs` |
| A3 | **Sub-millisecond Latency Routing**<br/>P99 overhead routing di bawah 1ms (<30MB RAM footprint) | SRE & FinOps | **VERIFIED** | Telemetri kernel di `tests/phase35_live_data_plane_test.rs` |
| A4 | **Native Pure-Rust TLS 1.3 & mTLS Ingress**<br/>Enkripsi transport tanpa reverse proxy eksternal | CISO & Security | **PARTIAL** | mTLS abstraction & handshake flags aktif; modul auto-ACME (Let's Encrypt) direncanakan via rustls-acme |

---

### B. Zero-Trust Security, ABAC & Data Loss Prevention (DLP)
| No | Ekspektasi Enterprise | Persona Utama | Status | Bukti Verifikasi / Keterangan |
| :--- | :--- | :--- | :--- | :--- |
| B1 | **Zero-Knowledge Credential Broker**<br/>LLM tidak pernah melihat atau menyimpan raw cloud secrets | CISO | **VERIFIED** | In-memory loopback proxy & `VaultSecretStore` terbukti di `tests/phase2_zero_trust_test.rs` |
| B2 | **Dynamic Payload ABAC & Anti-DDL**<br/>Evaluator OPA/Cedar dengan batasan nominal dan pencegahan mutasi SQL | Security Eng | **VERIFIED** | `OpaPolicyEngine` memblokir mutasi DDL, terbukti di `tests/phase2_zero_trust_test.rs` |
| B3 | **Sub-ms Bidirectional Real-Time DLP**<br/>PCI-DSS 4.0 PAN masking (`411111******1111`), HIPAA PHI, & key truncation | CISO & Auditor | **VERIFIED** | `PresidioDlpPipeline` recursive redaction terbukti di `tests/phase3_dlp_guardrails_test.rs` |
| B4 | **Prompt Poisoning & Threat Enrichment**<br/>Deteksi injeksi prompt pada deskripsi tool dan scanning URL | Security Eng | **VERIFIED** | `DefaultPoisonScanner` terbukti di `tests/enterprise_governance_test.rs` |
| B5 | **In-Situ Analytical Enclave (Zero-Egress)**<br/>Server-side analytical clean room query dataset SQLite/CSV/Parquet | Data Scientist | **VERIFIED** | Real dataset engine & zero-egress validation terbukti di `tests/phase43_real_in_situ_analytical_enclave_test.rs` |

---

### C. Zero-Downtime Control Plane & Day-2 Operations
| No | Ekspektasi Enterprise | Persona Utama | Status | Bukti Verifikasi / Keterangan |
| :--- | :--- | :--- | :--- | :--- |
| C1 | **Zero-Downtime Dynamic Reconciler**<br/>Registrasi backend, reload policy, dan kuota via RCU/RwLock tanpa restart | Platform SRE | **VERIFIED** | Atomic state swap terbukti di `tests/phase34_embedded_ui_control_suite_test.rs` |
| C2 | **Distributed HA State & Redis Cluster**<br/>Multi-pod state synchronization, rate limiting Lua, dan circuit breaker | Architect & SRE | **VERIFIED** | `RedisStateBackend` & health probes terbukti di `tests/phase1_distributed_test.rs` |
| C3 | **Declarative Kubernetes CRD Operator**<br/>GitOps controller untuk Custom Resource `PropylonBackend` & `PropylonPolicy` | Platform SRE | **VERIFIED** | Parsing, validasi skema & zero-restart reconciliation terbukti di `tests/phase45_kubernetes_crd_operator_test.rs` |
| C4 | **Distributed Cluster Sync Bus**<br/>Sinkronisasi multi-node dengan rolling checksum SHA-256 anti-split-brain | Architect | **VERIFIED** | Sync bus pub/sub terbukti di `tests/phase1_distributed_test.rs` |

---

### D. Human-in-the-Loop (HITL) & Safety Safeguards
| No | Ekspektasi Enterprise | Persona Utama | Status | Bukti Verifikasi / Keterangan |
| :--- | :--- | :--- | :--- | :--- |
| D1 | **Suspended High-Risk Action Gate**<br/>Penahanan aksi destruktif ke antrean pending HITL | Security Eng | **VERIFIED** | `ActionApprovalGate` terbukti di `tests/phase35_live_data_plane_test.rs` |
| D2 | **Cryptographic HMAC Approval Resolution**<br/>Verifikasi tiket persetujuan anti-pemalsuan | Auditor | **VERIFIED** | Tiket HMAC SHA-256 terbukti di `tests/phase35_live_data_plane_test.rs` |
| D3 | **Multi-Channel Dispatcher (Slack / Teams)**<br/>Pengiriman kartu interaktif ber-HMAC ke Slack & Teams | IT Operations | **VERIFIED** | Formatter BlockKit & Adaptive Cards terbukti di `tests/phase44_multi_channel_hitl_webhook_dispatcher_test.rs` |

---

### E. FinOps Multi-Tenancy & Tamper-Evident SIEM Audit
| No | Ekspektasi Enterprise | Persona Utama | Status | Bukti Verifikasi / Keterangan |
| :--- | :--- | :--- | :--- | :--- |
| E1 | **Hard Budget Freeze Cutoff (100%)**<br/>Pemutusan instan autonomous loop saat kuota habis dan peringatan soft 80% | VP Engineering | **VERIFIED** | `HardFreezeQuota` terbukti di `tests/phase5_finops_quota_test.rs` |
| E2 | **Departmental Chargeback & Prometheus**<br/>Alokasi biaya per departemen dan metrik open standard | FinOps Lead | **VERIFIED** | Prometheus exporter terbukti di `tests/phase5_finops_quota_test.rs` |
| E3 | **Cryptographic SHA-256 Hash Chained SIEM**<br/>Log non-repudiasi terhubung rantai kriptografi | CISO & Auditor | **VERIFIED** | `HashChainSequencer` terbukti di `tests/phase4_siem_audit_test.rs` |
| E4 | **SOC 2 Type II Proof Pack Export**<br/>Ekspor bundel bukti kepatuhan resmi format JSON | Compliance Off | **VERIFIED** | `export_soc2_report` CLI/REST terbukti di `tests/phase4_siem_audit_test.rs` |

---

### F. Embedded Web UI Control Plane (:8485)
| No | Ekspektasi Antarmuka | Status | Bukti Implementasi / Lokasi |
| :--- | :--- | :--- | :--- |
| F1 | **Tab 1: Overview & Metrics**<br/>Telemetri RAM fisik aktual, uptime, P99 latency, mTLS, policy tier | **VERIFIED** | `ui/src/lib/OverviewTab.svelte` terhubung ke `GET /api/v1/overview` |
| F2 | **Tab 2: Backends**<br/>Manajemen siklus hidup backend, circuit breaker, pendaftaran runtime | **VERIFIED** | `ui/src/lib/BackendsTab.svelte` terhubung ke `GET/POST /api/v1/backends` |
| F3 | **Tab 3: Tools Catalog & Playground**<br/>Filter All/Live/Templates, progressive disclosure L0/L1/L2, eksekusi jujur | **VERIFIED** | `ui/src/lib/ToolsTab.svelte` terhubung ke `GET /api/v1/tools` & `/call` |
| F4 | **Tab 4: Topology & Config Editor**<br/>Monospace YAML/JSON editor, preflight validation, hot-reload, backup otomatis | **VERIFIED** | `ui/src/lib/ConfigTab.svelte` terhubung ke `GET/POST /api/v1/config/*` |
| F5 | **Tab 5: Policy Control**<br/>Hot-swap tier Dev/Hybrid/Strict tanpa downtime layanan | **VERIFIED** | `ui/src/lib/PolicyTab.svelte` terhubung ke `GET/POST /api/v1/policy/tier` |
| F6 | **Tab 6: DLP Security Stream**<br/>Audit feed pencegahan kebocoran PII/PCI real-time | **VERIFIED** | `ui/src/lib/DlpTab.svelte` terhubung ke `GET /api/v1/dlp/events` |
| F7 | **Tab 7: Suspended HITL**<br/>Antrean aksi sensitif dengan tombol Approve/Reject ber-HMAC | **VERIFIED** | `ui/src/lib/HitlTab.svelte` terhubung ke `GET/POST /api/v1/hitl/*` |
| F8 | **Tab 8: Tenant FinOps**<br/>Visualisasi konsumsi token, alokasi biaya, dan status hard freeze | **VERIFIED** | `ui/src/lib/FinopsTab.svelte` terhubung ke `GET /api/v1/finops` |
| F9 | **Tab 9: SOC 2 Audit**<br/>Root hash SHA-256 dan ekspor proof pack JSON satu klik | **VERIFIED** | `ui/src/lib/AuditTab.svelte` terhubung ke `GET /api/v1/audit/soc2` |
| F10 | **UI Themes & Invariants**<br/>Zinc, Slate, Neutral, OLED, dark/light mode, zero emoticons | **VERIFIED** | `ui/src/App.svelte` & Tailwind tokens |
| F11 | **Interactive Topology Flow Canvas (@xyflow/svelte)**<br/>Visualisasi graf interaktif DAG alur request, pulse circuit breaker, dan inspector drawer | **ROADMAP (Phase 46-48)** | Direncanakan di `docs/roadmap/46_...` - `48_...` |

---

## 2. Strategi Retensi Permanen (Agar Diingat Terus Kedepannya)

Agar seluruh ekspektasi end-to-end ini tidak pernah hilang atau terlupakan saat context window berganti:

1. **Penyimpanan Primer**: Berkas ini disimpan di [docs/ENTERPRISE_EXPECTATIONS_MATRIX.md](file:///root/projects/propylon/docs/ENTERPRISE_EXPECTATIONS_MATRIX.md) sebagai kontrak hidup repositori.
2. **Penegakan Invariant Global**: [GEMINI.md](file:///root/GEMINI.md) memuat klausul wajib (Principle 11) yang mengikat setiap sesi AI Agent untuk menyelaraskan roadmap dan pengujian dengan matriks ini.
3. **Automated Gauntlet Enforcement**: Skrip [scripts/gauntlet.sh](file:///root/projects/propylon/scripts/gauntlet.sh) dan scanner tata kelola menjamin bahwa seluruh verifikasi dilakukan secara empiris tanpa silent mocks.
