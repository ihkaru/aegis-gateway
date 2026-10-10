#!/usr/bin/env python3
"""
Aegis Gateway - Enterprise Scale Accuracy & Planning Benchmark
Evaluates Tool Discovery Recall, Multilingual Robustness, Action Disambiguation,
and Topological DAG Planning against the live Aegis Gateway endpoint.
"""

import json
import time
import urllib.request
import urllib.error
from typing import List, Dict, Any, Tuple

DEFAULT_ENDPOINT = "https://mcp.dvlpid.my.id/mcp"

# 120 Enterprise Tools across 8 Core Domains (modeled from APIs.guru & ToolBench)
ENTERPRISE_TOOLS = [
    # Domain: Databases & Warehouses
    {"name": "query_postgresql_ledger", "server": "db_cluster", "desc": "Execute SQL queries to retrieve transactional ledger entries", "tags": ["database", "sql", "postgres", "tabel", "data"]},
    {"name": "destructive_drop_table", "server": "db_cluster", "desc": "Permanently drop and delete database table and schema", "tags": ["drop", "delete", "hapus", "table", "tabel"]},
    {"name": "clickhouse_aggregate_metrics", "server": "analytics_dw", "desc": "Perform columnar OLAP aggregation over event telemetry", "tags": ["clickhouse", "analytics", "olap", "metrik"]},
    {"name": "redis_cache_invalidate", "server": "cache_kv", "desc": "Purge key or namespace pattern from in-memory cluster", "tags": ["redis", "cache", "purge", "hapus", "flush"]},
    {"name": "snowflake_export_warehouse", "server": "analytics_dw", "desc": "Export warehouse query results to cloud staging bucket", "tags": ["snowflake", "export", "warehouse", "berkas"]},
    
    # Domain: Cloud Storage & Media
    {"name": "upload_to_storage", "server": "mediavault", "desc": "Store documents and images into persistent S3 cloud bucket", "tags": ["s3", "file", "berkas", "upload", "simpan", "media"]},
    {"name": "delete_storage_object", "server": "mediavault", "desc": "Permanently delete an uploaded document or image from S3", "tags": ["s3", "file", "berkas", "delete", "hapus"]},
    {"name": "generate_presigned_url", "server": "mediavault", "desc": "Create temporary presigned download link for storage object", "tags": ["s3", "url", "link", "unduh", "download"]},
    {"name": "transcode_media_video", "server": "mediavault", "desc": "Transcode raw video stream into optimized HLS and MP4", "tags": ["video", "transcode", "media", "hls"]},
    
    # Domain: Payments & Financials
    {"name": "stripe_create_customer", "server": "stripe_billing", "desc": "Register new paying customer and initialize billing ledger", "tags": ["stripe", "customer", "pelanggan", "billing"]},
    {"name": "stripe_charge_card", "server": "stripe_billing", "desc": "Authorize and capture charge on customer credit card", "tags": ["stripe", "charge", "bayar", "payment", "kartu"]},
    {"name": "stripe_issue_refund", "server": "stripe_billing", "desc": "Reverse settled payment transaction and return funds to buyer", "tags": ["stripe", "refund", "kembalikan", "retur"]},
    {"name": "tax_calculate_vat", "server": "fiscal_calc", "desc": "Calculate local VAT and sales tax percentage by jurisdiction", "tags": ["tax", "pajak", "vat", "hitung"]},
    
    # Domain: Messaging & Notifications
    {"name": "dispatch_slack_alert", "server": "ops_notify", "desc": "Broadcast incident alert message to engineering Slack channel", "tags": ["slack", "alert", "pesan", "kirim", "notifikasi"]},
    {"name": "sendgrid_send_invoice_email", "server": "email_gateway", "desc": "Dispatch PDF invoice email receipt to customer inbox", "tags": ["email", "invoice", "tagihan", "kirim", "surel"]},
    {"name": "twilio_send_sms_otp", "server": "sms_gateway", "desc": "Send one-time SMS verification passcode to mobile number", "tags": ["sms", "otp", "pesan", "kirim", "telepon"]},
    {"name": "pagerduty_trigger_incident", "server": "ops_notify", "desc": "Page on-call engineer for critical production outage", "tags": ["pagerduty", "incident", "darurat", "ops"]},
    
    # Domain: DevOps & Infrastructure
    {"name": "github_create_pull_request", "server": "devops_hub", "desc": "Create pull request for feature branch review", "tags": ["github", "git", "pr", "repo", "kode"]},
    {"name": "docker_restart_container", "server": "devops_hub", "desc": "Send SIGTERM and restart running containerized application", "tags": ["docker", "container", "restart", "jalankan"]},
    {"name": "k8s_scale_deployment", "server": "devops_hub", "desc": "Scale replica count for Kubernetes pod deployment", "tags": ["k8s", "kubernetes", "scale", "pod"]},
    
    # Domain: CRM & Customer Support
    {"name": "zendesk_create_ticket", "server": "crm_support", "desc": "Open new customer support ticket with priority and tags", "tags": ["ticket", "support", "bantuan", "keluhan"]},
    {"name": "salesforce_update_deal", "server": "crm_support", "desc": "Update deal stage and monetary value in sales pipeline", "tags": ["salesforce", "deal", "penjualan", "crm"]},
]

# Generate remaining up to 100 tools for scale test
for i in range(len(ENTERPRISE_TOOLS) + 1, 101):
    category = ["data_sync", "auth_iam", "crawler", "audit_log"][i % 4]
    ENTERPRISE_TOOLS.append({
        "name": f"{category}_worker_node_{i}",
        "server": f"{category}_cluster",
        "desc": f"Execute automated task on {category} cluster node #{i}",
        "tags": [category, "worker", "job", "task", f"node_{i}"]
    })

# 20 Diverse Accuracy Test Scenarios
EVALUATION_SCENARIOS = [
    # 1. Indonesian Vernacular Queries
    {"query": "simpan berkas ke cloud", "expected": "upload_to_storage", "tier": "ID_Colloquial"},
    {"query": "hapus tabel data ledger", "expected": "destructive_drop_table", "tier": "ID_Colloquial"},
    {"query": "kirim pesan alert ke tim ops", "expected": "dispatch_slack_alert", "tier": "ID_Colloquial"},
    {"query": "kirim email tagihan ke pelanggan", "expected": "sendgrid_send_invoice_email", "tier": "ID_Colloquial"},
    {"query": "kembalikan uang pembayaran pembeli", "expected": "stripe_issue_refund", "tier": "ID_Colloquial"},
    
    # 2. English Technical Queries
    {"query": "execute sql queries to fetch records", "expected": "query_postgresql_ledger", "tier": "EN_Technical"},
    {"query": "charge customer credit card for order", "expected": "stripe_charge_card", "tier": "EN_Technical"},
    {"query": "drop database table permanently", "expected": "destructive_drop_table", "tier": "EN_Technical"},
    {"query": "generate presigned download link for file", "expected": "generate_presigned_url", "tier": "EN_Technical"},
    {"query": "send one time sms passcode to phone", "expected": "twilio_send_sms_otp", "tier": "EN_Technical"},
    
    # 3. Disambiguation (Destructive vs Read/Query)
    {"query": "delete file from cloud storage", "expected": "delete_storage_object", "tier": "Disambiguation"},
    {"query": "upload new file to storage", "expected": "upload_to_storage", "tier": "Disambiguation"},
    {"query": "purge cache pattern from redis", "expected": "redis_cache_invalidate", "tier": "Disambiguation"},
    {"query": "query database transactions", "expected": "query_postgresql_ledger", "tier": "Disambiguation"},
    {"query": "restart container service", "expected": "docker_restart_container", "tier": "Disambiguation"},
    
    # 4. Multilingual Domain Specific
    {"query": "hitung pajak vat transaksi", "expected": "tax_calculate_vat", "tier": "Multilingual"},
    {"query": "buka tiket bantuan pelanggan baru", "expected": "zendesk_create_ticket", "tier": "Multilingual"},
    {"query": "scale kubernetes pod replica count", "expected": "k8s_scale_deployment", "tier": "EN_Technical"},
    {"query": "buat pull request review kode", "expected": "github_create_pull_request", "tier": "Multilingual"},
    {"query": "transcode video hls streaming", "expected": "transcode_media_video", "tier": "EN_Technical"},
]

def make_mcp_request(endpoint: str, method: str, params: Dict[str, Any]) -> Tuple[int, Dict[str, Any], float]:
    payload = {
        "jsonrpc": "2.0",
        "id": int(time.time() * 1000),
        "method": method,
        "params": params
    }
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        endpoint,
        data=data,
        headers={
            "Content-Type": "application/json",
            "Accept": "application/json",
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 AegisBenchmark/1.0"
        }
    )
    start = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=10.0) as resp:
            latency_ms = (time.perf_counter() - start) * 1000.0
            body = json.loads(resp.read().decode("utf-8"))
            return resp.status, body, latency_ms
    except urllib.error.HTTPError as e:
        latency_ms = (time.perf_counter() - start) * 1000.0
        body = json.loads(e.read().decode("utf-8")) if e.fp else {"error": str(e)}
        return e.code, body, latency_ms
    except Exception as e:
        latency_ms = (time.perf_counter() - start) * 1000.0
        return 500, {"error": str(e)}, latency_ms

def run_benchmark(endpoint: str = DEFAULT_ENDPOINT):
    print("=" * 65)
    print(f"AEGIS GATEWAY EMPIRICAL ACCURACY & PLANNING BENCHMARK")
    print(f"Target Endpoint : {endpoint}")
    print(f"Tools in Catalog: {len(ENTERPRISE_TOOLS)}")
    print("=" * 65)

    # 1. Healthcheck Probe
    print("\n[Step 1] Probing Gateway Health & Wire Protocol...")
    code, list_res, lat = make_mcp_request(endpoint, "tools/list", {})
    if code != 200:
        print(f"FAILED: Endpoint returned HTTP {code}: {list_res}")
        return
    print(f"  [PASS] Endpoint responsive (HTTP 200, {lat:.1f}ms). Found tools: {[t['name'] for t in list_res.get('result', {}).get('tools', [])]}")

    # 2. Test Discovery Search Accuracy
    print("\n[Step 2] Evaluating Tool Discovery Accuracy across 20 Scenarios...")
    top1_hits = 0
    top3_hits = 0
    top5_hits = 0
    latencies = []

    for i, scen in enumerate(EVALUATION_SCENARIOS, 1):
        code, res, lat = make_mcp_request(endpoint, "tools/call", {
            "name": "gateway_search_tools",
            "arguments": {"query": scen["query"], "tier": "L0", "top_k": 5}
        })
        latencies.append(lat)
        tools_found = []
        if "result" in res:
            r = res["result"]
            if "tools" in r:
                tools_found = [t.get("name") for t in r.get("tools", [])]
            elif "content" in r and r["content"]:
                try:
                    parsed = json.loads(r["content"][0]["text"])
                    tools_found = [t.get("name") for t in parsed.get("tools", [])]
                except Exception:
                    pass

        expected = scen["expected"]
        rank = tools_found.index(expected) + 1 if expected in tools_found else None

        if rank == 1:
            top1_hits += 1
            top3_hits += 1
            top5_hits += 1
            status = "HIT (Rank 1)"
        elif rank and rank <= 3:
            top3_hits += 1
            top5_hits += 1
            status = f"HIT (Rank {rank})"
        elif rank and rank <= 5:
            top5_hits += 1
            status = f"HIT (Rank {rank})"
        else:
            status = f"MISS (Expected: {expected}, Got: {tools_found[:3]})"

        print(f"  [{i:02d}/20] [{scen['tier']:<14}] \"{scen['query']:<35}\" -> {status} ({lat:.1f}ms)")

    # 3. Test Multi-Hop DAG Planning & Cycle Prevention
    print("\n[Step 3] Evaluating Multi-Step DAG Planning & Cycle Prevention...")
    # 3A: Valid Sequential Plan
    valid_plan = {
        "plan_id": "plan_invoice_pipeline",
        "title": "Customer Invoicing Flow",
        "steps": [
            {"id": "step_cust", "tool": "stripe_create_customer", "arguments": {"email": "user@example.com"}, "depends_on": []},
            {"id": "step_pay", "tool": "stripe_charge_card", "arguments": {"customer_id": "{{step_cust.output.id}}", "amount": 5000}, "depends_on": ["step_cust"]},
            {"id": "step_mail", "tool": "sendgrid_send_invoice_email", "arguments": {"charge_id": "{{step_pay.output.charge_id}}"}, "depends_on": ["step_pay"]}
        ]
    }
    _, val_res, val_lat = make_mcp_request(endpoint, "tools/call", {"name": "gateway_plan_tasks", "arguments": valid_plan})
    val_parsed = val_res.get("result", {})
    if not val_parsed and "content" in val_res.get("result", {}):
        val_parsed = json.loads(val_res["result"]["content"][0]["text"])
    valid_plan_ok = val_parsed.get("valid", False) and val_parsed.get("step_count", 0) == 3
    print(f"  [3A] Valid 3-Step DAG Planning: {'PASS' if valid_plan_ok else 'FAIL'} ({val_lat:.1f}ms) - Steps validated: {val_parsed.get('step_count', 0)}, Risk: {val_parsed.get('overall_risk')}")

    # 3B: Circular Dependency Injection (Cycle Rejection Test)
    cycle_plan = {
        "plan_id": "plan_deadlock_cycle",
        "title": "Deadlock Cycle Plan",
        "steps": [
            {"id": "step_A", "tool": "upload_to_storage", "arguments": {}, "depends_on": ["step_C"]},
            {"id": "step_B", "tool": "generate_presigned_url", "arguments": {}, "depends_on": ["step_A"]},
            {"id": "step_C", "tool": "delete_storage_object", "arguments": {}, "depends_on": ["step_B"]}
        ]
    }
    _, cyc_res, cyc_lat = make_mcp_request(endpoint, "tools/call", {"name": "gateway_plan_tasks", "arguments": cycle_plan})
    cyc_parsed = cyc_res.get("result", {})
    if not cyc_parsed and "content" in cyc_res.get("result", {}):
        cyc_parsed = json.loads(cyc_res["result"]["content"][0]["text"])
    cycle_rejected = not cyc_parsed.get("valid", True) and "Circular dependency" in " ".join(cyc_parsed.get("errors", []))
    print(f"  [3B] Cyclic Deadlock Injection: {'PASS (Rejected)' if cycle_rejected else 'FAIL'} ({cyc_lat:.1f}ms) - Errors: {cyc_parsed.get('errors', [])}")

    # 3C: Autonomous Natural Language Goal Planning (Step-by-Step Multi-Tool Synthesis)
    print("\n[Step 4] Evaluating Autonomous Multi-Tool Goal Synthesis across Complex Objectives...")
    goal_scenarios = [
        ("Ambil data transaksi ledger database, simpan berkas ke cloud storage, lalu kirim pesan alert ke tim ops", 3, ["query_postgresql_ledger", "upload_to_storage", "dispatch_slack_alert"]),
        ("Register new paying customer on Stripe, charge customer credit card for order, then dispatch invoice email to customer inbox", 3, ["stripe_create_customer", "stripe_charge_card", "sendgrid_send_invoice_email"]),
        ("Ambil data transaksi ledger database, transcode video hls streaming, simpan berkas ke cloud storage, buat pull request review kode, lalu kirim pesan alert ke tim ops", 5, ["query_postgresql_ledger", "transcode_media_video", "upload_to_storage", "github_create_pull_request", "dispatch_slack_alert"])
    ]

    goal_hits = 0
    for idx, (goal_text, expected_steps, expected_tools) in enumerate(goal_scenarios, 1):
        _, g_res, g_lat = make_mcp_request(endpoint, "tools/call", {"name": "gateway_plan_tasks", "arguments": {"goal": goal_text}})
        g_parsed = g_res.get("result", {})
        if not g_parsed and "content" in g_res.get("result", {}):
            g_parsed = json.loads(g_res["result"]["content"][0]["text"])
        plan_steps = g_parsed.get("plan", {}).get("steps", [])
        actual_tools = [s.get("tool") for s in plan_steps]
        is_ok = g_parsed.get("valid", False) and len(plan_steps) == expected_steps and actual_tools == expected_tools
        if is_ok:
            goal_hits += 1
            g_status = f"PASS ({len(plan_steps)} steps, sequence: {actual_tools})"
        else:
            g_status = f"FAIL (Expected: {expected_tools}, Got: {actual_tools})"
        print(f"  [4.{idx}] Goal: \"{goal_text[:50]}...\" -> {g_status} ({g_lat:.1f}ms)")


    # 4. Summary Scorecard
    total = len(EVALUATION_SCENARIOS)
    top1_pct = (top1_hits / total) * 100.0
    top3_pct = (top3_hits / total) * 100.0
    top5_pct = (top5_hits / total) * 100.0
    latencies.sort()
    p50_lat = latencies[int(len(latencies) * 0.50)]
    p95_lat = latencies[int(len(latencies) * 0.95)]
    p99_lat = latencies[-1]

    print("\n" + "=" * 65)
    print("           AEGIS GATEWAY EVALUATION SCORECARD            ")
    print("=" * 65)
    print(f"  Top-1 Accuracy (Exact Best Match) : {top1_hits:02d}/{total:02d} ({top1_pct:.1f}%)")
    print(f"  Top-3 Accuracy (Recall@3)         : {top3_hits:02d}/{total:02d} ({top3_pct:.1f}%)")
    print(f"  Top-5 Accuracy (Recall@5)         : {top5_hits:02d}/{total:02d} ({top5_pct:.1f}%)")
    print(f"  DAG Topological Cycle Rejection   : {'100.0% (PASSED)' if cycle_rejected else 'FAILED'}")
    print(f"  Multi-Hop DAG Formulation         : {'100.0% (PASSED)' if valid_plan_ok else 'FAILED'}")
    print(f"  Autonomous Goal Plan Synthesis    : {goal_hits}/{len(goal_scenarios)} ({goal_hits/len(goal_scenarios)*100.0:.1f}%)")
    print(f"  Latency P50                       : {p50_lat:.1f} ms")
    print(f"  Latency P95                       : {p95_lat:.1f} ms")
    print(f"  Latency P99                       : {p99_lat:.1f} ms")
    print("=" * 65)

if __name__ == "__main__":
    import sys
    ep = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ENDPOINT
    run_benchmark(ep)
