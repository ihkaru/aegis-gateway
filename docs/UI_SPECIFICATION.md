# Aegis Gateway: Embedded Web UI Control Plane Specification

> [!IMPORTANT]
> **Single Source of Truth & Governance Invariant**:
> Dokumen ini adalah spesifikasi wajib (*living contract*) untuk Embedded Web UI Control Plane Aegis Gateway.
> Setiap penambahan fitur, perubahan komponen antarmuka, perbaikan UX, atau ekspektasi baru terkait Web UI **WAJIB dicatat dan diperbarui di dokumen ini**. Ketentuan ini ditegakkan dalam `/root/GEMINI.md`.

---

## 1. Prinsip Desain & Tata Kelola Antarmuka

1. **Zero-Mock & Honest Preflight Invariant (Non-Negotiable)**:
   - Dilarang keras menampilkan data dummy statis atau memalsukan eksekusi backend.
   - **Status LIVE**: Tool yang terhubung ke backend riil yang sedang aktif (`aegis_local_engine`, `sqlite_ledger`). Eksekusi dijalankan langsung ke mesin eksekusi (misal: query SQL riil ke file database `data/aegis_local.db`).
   - **Status TEMPLATE**: Skema kontrak integrasi enterprise resmi (Postgres, Stripe, S3, Docker, Slack, dll.) yang didaftarkan untuk autonomous goal planning dan validasi preflight guardrail.
   - **Eksekusi Jujur**: Jika tool berstatus TEMPLATE dijalankan tanpa backend upstream yang terpasang, gateway mengembalikan status jujur `PREFLIGHT_GUARDRAIL_ONLY` dengan keterangan `status: "unattached"`, bukan simulasi data palsu.
2. **Zero-Emoticon & Professional Aesthetics**:
   - Dilarang menggunakan emoji/emotikon teks atau Unicode di seluruh UI.
   - Seluruh ikon wajib menggunakan SVG komponen dari `lucide-svelte` (`Activity`, `Server`, `Terminal`, `Lock`, `Sliders`, `Shield`, `Clock`, `Layers`, `FileText`, `CheckCircle2`, `AlertTriangle`).
3. **Hard Constraint Batas Baris**:
   - Seluruh file komponen Svelte di `ui/src/` wajib mematuhi batas maksimal `wc -l <= 350`.

---

## 2. Struktur Navigasi & Spesifikasi Menu

### Tab 1: Overview & Metrics (`OverviewTab.svelte`)
- **Tujuan**: Memantau kesehatan operasional, throughput, dan telemetri memori gateway.
- **REST Binding**: `GET /api/v1/overview`.
- **Ekspektasi Data**:
  - `status`: Status kesehatan (`HEALTHY`).
  - `uptime_secs`: Waktu aktif gateway dalam detik.
  - `memory_rss_mb`: Penggunaan memori fisik aktual dari kernel (`/proc/self/status`).
  - `p99_latency_ms`: Latensi P99 sub-milidetik.
  - `active_backends` & `total_tools`: Jumlah backend dan tool terdaftar.
  - `policy_tier`: Tier keamanan aktif (`Hybrid`, `Dev`, atau `Strict`).
  - `mtls_enforced`: Indikator keamanan transport mTLS.

### Tab 2: Backends (`BackendsTab.svelte`)
- **Tujuan**: Mengelola siklus hidup upstream MCP servers secara dinamis.
- **REST Binding**: `GET /api/v1/backends`, `POST /api/v1/backends`, `DELETE /api/v1/backends/:name`.
- **Ekspektasi Data & Aksi**:
  - Menampilkan daftar backend aktif dengan tipe `Subprocess` (stdio) atau `RemoteHttp` (SSE/HTTP).
  - Status koneksi, circuit breaker (`CLOSED`/`OPEN`), jumlah tool, dan latensi.
  - Modal penambahan backend baru secara runtime tanpa restart service.
  - Pencabutan backend (*deregister*) instan.

### Tab 3: Tools & Playground (`ToolsTab.svelte`)
- **Tujuan**: Menjelajahi katalog alat terdaftar dan menjalankan eksekusi interaktif.
- **REST Binding**: `GET /api/v1/tools`, `POST /api/v1/tools/call`.
- **Ekspektasi Data & Aksi**:
  - Filter pencarian teks instan dan filter kategori: `All`, `Live Active`, dan `Templates`.
  - Progressive Disclosure tiga tingkat token: `L0 Purpose` (~50 tokens), `L1 Signature`, dan `L2 Full Schema`.
  - Badge visual pembeda: `LIVE` (hijau) vs `TEMPLATE` (abu-abu/biru).
  - Playground editor argumen JSON dengan template otomatis sesuai skema tool.
  - Tombol eksekusi dengan feedback jujur:
    - `LIVE EXECUTED` (hijau): Eksekusi riil pada backend lokal/upstream.
    - `PREFLIGHT VERIFIED` (kuning): Evaluasi guardrail ABAC/DLP untuk tool unattached.

### Tab 4: Topology & Config (`TopologyTab.svelte` & `ConfigTab.svelte`)
- **Tujuan**: Visualisasi DAG topologi interaktif berbasis Svelte Flow (`@xyflow/svelte`) dan editor deklaratif in-browser untuk `aegis.yaml`.
- **REST Binding**: `GET /api/v1/topology/graph`, `GET /api/v1/config/raw`, `POST /api/v1/config/apply`.
- **Ekspektasi Data & Aksi**:
  - **Svelte Flow Canvas**: Graf DAG interaktif (pan, zoom, minimap) memetakan alur request: `Clients -> Ingress (:8484) -> Security Filters -> Tool Router -> MCP Backends & Integration Channels`.
  - **Live Circuit Breaker Pulse**: Indikator status reaktif real-time pada simpul backend (Hijau = Normal/Closed, Merah = Tripped/Open).
  - **Interactive Node Drawer**: Klik simpul untuk inspeksi diagnostik, latensi P99, dan tombol reset circuit breaker.
  - **Declarative YAML Editor**: Monospace code editor dengan preflight syntax validation dan atomic zero-downtime hot-reload.

### Tab 5: Policy Control (`PolicyTab.svelte`)
- **Tujuan**: Manajemen otorisasi ABAC dan pembatasan argumen destruktif.
- **REST Binding**: `GET /api/v1/policy/tier`, `POST /api/v1/policy/tier`.
- **Ekspektasi Data & Aksi**:
  - Pemilihan tier kebijakan: `Dev` (permissive), `Hybrid` (dynamic bounds), `Strict` (zero-trust, anti-DDL).
  - Peralihan tier instan tanpa downtime.

### Tab 6: DLP Security Stream (`DlpTab.svelte`)
- **Tujuan**: Inspeksi pencegahan kebocoran data sensitif (PII/PCI/API Key) dua arah.
- **REST Binding**: `GET /api/v1/dlp/events`.
- **Ekspektasi Data**:
  - Feed pemantauan data keluar-masuk dengan kepatuhan PCI-DSS 4.0 PAN masking (`411111******1111`), HIPAA PHI, dan SHA-256 API key truncation.

### Tab 7: Suspended HITL (`HitlTab.svelte`)
- **Tujuan**: Penanganan aksi sensitif yang tertahan (*Human-in-the-Loop*).
- **REST Binding**: `GET /api/v1/hitl/queue`, `POST /api/v1/hitl/resolve`.
- **Ekspektasi Data & Aksi**:
  - Antrean tiket verifikasi persetujuan manusia.
  - Opsi tindakan: `Approve` (menerbitkan tiket izin eksekusi HMAC) atau `Reject`.

### Tab 8: Tenant FinOps (`FinopsTab.svelte`)
- **Tujuan**: Pemantauan kuota token dan pencegahan pemborosan biaya multi-tenant.
- **REST Binding**: `GET /api/v1/finops`.
- **Ekspektasi Data**:
  - Penggunaan token per penyewa, alokasi biaya (*chargeback*), peringatan soft (80%), dan hard freeze cutoff (100%).

### Tab 9: SOC 2 Audit (`AuditTab.svelte`)
- **Tujuan**: Pembuktian audit non-repudiasi dan kepatuhan regulasi.
- **REST Binding**: `GET /api/v1/audit/soc2`.
- **Ekspektasi Data & Aksi**:
  - Tampilan root checkpoint hash SHA-256 hash-chain.
  - Status pemenuhan kontrol Trust Services (CC6.1, CC6.6, CC6.7, CC7.1).
  - Ekspor laporan audit resmi (JSON proof pack).

---

## 3. Tema & Preferensi Visual
- **Pilihan Tema**: Zinc, Slate, Neutral, dan OLED (pure black `#000000` untuk panel AMOLED).
- **Mode Kontras**: Dukungan penuh Dark Mode dan Light Mode melalui toggle header global.
