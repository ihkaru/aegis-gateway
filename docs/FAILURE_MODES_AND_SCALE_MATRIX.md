# Enterprise Failure Modes & Concurrency Scale Matrix (20,000+ Users)

## 1. Executive Context & Concurrency Profile

When deployed at scale across large enterprise organizations (20,000+ employees), AI agents (Claude Code, Cursor, Antigravity, and autonomous bots) generate distinct traffic patterns fundamentally different from traditional REST APIs:

- **Active Concurrency**: 3,000 to 6,000 concurrent agent sessions during peak business hours.
- **Traffic Intensity**: 500 to 2,000 continuous requests per second (RPS), with burst spikes reaching 5,000 RPS.
- **Payload Characteristics**: Variable payload sizes ranging from sub-kilobyte JSON-RPC calls to multi-megabyte analytical queries and bidirectional streaming outputs.

Under this level of concurrency, cascading failure across distributed components is inevitable if boundaries and failure domains are not formally established.

---

## 2. The Failure Hierarchy (Cascading Domino Effect)

```
[Level 1: Upstream Stdio Subprocesses & External SaaS] --> FIRST TO COLLAPSE
     |
     v (Cascading Backpressure)
[Level 2: Distributed State Bus & Connection Pools]    --> SECONDARY STRAIN
     |
     v (Network & Memory Pressure)
[Level 3: Data Plane Stateless Ingress (:8484)]        --> MUST FAST-FAIL (<2ms)
     |
     v (Architecturally Decoupled via Control Bus)
[Level 4: Control Plane & Admin UI (:8485)]            --> NEVER DOWN (Citadel)
```

1. **Level 1 (First to Collapse)**: Unmanaged Python/Node.js Stdio subprocesses and rate-limited external third-party SaaS APIs.
2. **Level 2 (Secondary Strain)**: In-memory/Redis cluster sync and connection pools under heavy serialization and telemetry write volume.
3. **Level 3 (Fast-Fail Boundary)**: Stateless Tokio Data Plane Ingress. Must absorb shock, trip circuit breakers, and fast-fail in <2ms without hanging threads.
4. **Level 4 (Zero-Downtime Citadel)**: Control Plane, CRD Operator, and Embedded Admin UI on port `:8485`. Completely isolated from Data Plane saturation.

---

## 3. Failure Modes & Blast Radius Matrix

| Component / Layer | Failure Mode Scenario | Blast Radius & Symptoms | In-Scope: Aegis Gateway Defense | Out-of-Scope: Platform / Infra Responsibility |
| :--- | :--- | :--- | :--- | :--- |
| **Upstream Stdio Subprocesses** | Spawning thousands of child processes (Python/Node) exhausts OS process limits (`pid_max`, memory starvation, GIL locking). | Individual tools freeze; latencies spike to 30s+; child processes become zombies. | **Hermetic Process Reaping**: Time-bounded timeouts (default 3s), graceful SIGTERM/SIGKILL reaping, and distributed circuit breakers returning instant `-32603` fast-fail. | **Host Kernel Configuration**: Configure `ulimit -u`, kernel cgroup PID limits per pod, and container resource limits (`resources.limits.cpu/memory`). |
| **Upstream SaaS & Databases** | 20k employees trigger bulk requests exceeding vendor rate limits (429 Too Many Requests) or DB connection exhaustion (`max_connections`). | Upstream queries rejected; cascading transaction rollbacks across downstream workflows. | **Dynamic Rate Limiting & Circuit Breaker**: Token bucket quotas per tenant, proactive backoff, and fast refusal before egress traffic leaves gateway. | **Vendor Quota Management**: Purchasing higher tier enterprise rate limits from SaaS providers; setting up PgBouncer / RDS Proxy for database pooling. |
| **Data Plane Memory & Ingress** | Large analytical payloads (50MB+ CSV/Parquet) uploaded simultaneously by hundreds of agents, threatening Linux OOM-killer. | Pod memory balloons; risk of kernel termination of Aegis Gateway process. | **In-Situ Analytical Enclave & Chunk Streaming**: In-memory DuckDB zero-egress queries, streaming chunk limits, and strict payload size guardrails. | **Node Sizing & Autoscaling**: Provisioning nodes with adequate RAM buffers; configuring Kubernetes Horizontal Pod Autoscaler (HPA) targeting memory/CPU at 70%. |
| **Cluster State Sync (Redis / NATS)** | Millions of telemetry events, quotas, and distributed circuit breaker states saturate network sockets. | Cluster sync delay; potential split-brain drift in rate-limiting counters across pods. | **Local In-Memory Caching & Batch Sync**: Atomic RCU-style state, local in-memory fallback cache if Redis drops, and rolling SHA-256 checksums. | **Redis Cluster Sizing**: Redis Cluster topology with read replicas, high-bandwidth interconnects (10Gbps+), and Redis connection pooling configuration. |
| **Human-in-the-Loop (HITL) Gate** | Hundreds of destructive actions trigger approval cards, overwhelming Slack/Teams webhook rate limits. | Approval notifications delayed; task resolution suspended indefinitely. | **Multi-Channel Dispatch & Local Queue**: Round-robin channel targets, HMAC-signed local queuing, and durable resume timeout policies. | **Corporate Chat Infrastructure**: Webhook endpoint reliability, enterprise webhook rate limit exemptions, and corporate firewall egress rules. |
| **Embedded Web UI (:8485)** | High data plane tool call traffic on `:8484`. | Administrators worried UI will freeze or become inaccessible during incidents. | **Control Bus Isolation Invariant**: Admin UI runs on dedicated port `:8485` isolated from `:8484`; pre-bundled static assets via `rust-embed`. | **Ingress Ingress Controller**: Ensuring port `:8485` is routed exclusively to internal VPN/bastion and not exposed to public agent traffic. |

---

## 4. Responsibility Boundary: In-Scope vs. Out-of-Scope

### In-Scope (Aegis Gateway Architectural Guarantees)
- **Sub-2ms Fast-Fail Invariant**: Core proxy threads never block on degraded upstreams.
- **Circuit Breaker Automation**: Tripped backends are bypassed immediately with structured JSON-RPC errors.
- **Zero-Downtime Hot Swapping**: Adding, removing, or adjusting backends and policy tiers requires 0 pod restarts.
- **Bidirectional In-Memory Guardrails**: Sub-millisecond DLP, credential stripping, and DDL query refusals without third-party network dependencies.
- **Zero-Knowledge Loopback Proxying**: Ephemeral port bindings ensure models never touch raw secrets.

### Out-of-Scope (Enterprise Platform & SRE Guarantees)
- **Host Kernel Tuning**: SREs must configure `fs.file-max`, `net.core.somaxconn`, and `ulimit -n 65535` on container hosts.
- **External Network Latency**: Cloud WAN latency between gateway pods and third-party SaaS APIs.
- **Database Connection Pooling**: SREs must front shared enterprise databases with dedicated connection poolers (e.g. PgBouncer, ProxySQL).
- **Identity Provider High Availability**: High-availability clustering of upstream OIDC/SAML IdP servers (Okta, Keycloak, Entra ID).
- **Physical Hardware & Virtualization**: Hypervisor scheduling, cloud provider zone outages, and NIC hardware bandwidth saturation.

---

## 5. High-Concurrency SRE Verification Checklist

Before onboarding 20,000+ enterprise users:
- [ ] Verify `ulimit -n` is set to at least 65535 inside the container.
- [ ] Confirm Kubernetes HPA is configured with minimum 3 replicas and scaling threshold at 70% CPU/Memory.
- [ ] Verify Upstream Stdio processes have strict execution timeouts configured (<= 5000ms).
- [ ] Ensure Redis cluster backend has connection pool limits aligned with maximum gateway pod count.
- [ ] Confirm port `:8485` is bound to administrative internal ingress only.
- [ ] Run `bash scripts/gauntlet.sh` to confirm zero unhandled panics and 100% test pass rate.
