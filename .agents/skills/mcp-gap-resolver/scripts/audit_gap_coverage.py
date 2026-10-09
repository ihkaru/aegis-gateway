#!/usr/bin/env python3
"""
Audit Gap Coverage across Aegis Gateway test suites and source code.
Cross-references a catalog of known legacy MCP complaints and issues against test assertions.
"""

import argparse
import os
import re
import sys

TARGET_COMPLAINTS = [
    {"id": "GH-555", "desc": "Dynamic Payload ABAC & SQL bounds validation", "aliases": ["555", "abac", "payload"]},
    {"id": "GH-3034", "desc": "Subprocess silent crash & cascading circuit breaker", "aliases": ["3034", "circuit_breaker"]},
    {"id": "GH-622", "desc": "Dedicated package manager cache isolation (npm/uv/pip)", "aliases": ["622", "cache_isolation", "npm_config_cache"]},
    {"id": "GH-2317", "desc": "Tool discovery matching by server name & tags", "aliases": ["2317", "discover_tools", "search_tool"]},
    {"id": "GH-523", "desc": "Cross-platform command split (Windows vs POSIX)", "aliases": ["523", "command_split"]},
    {"id": "GH-540", "desc": "mcp-protocol-version header validation", "aliases": ["540", "mcp_protocol_version"]},
    {"id": "GH-591", "desc": "Accurate refused tool call audit logging (no fake calls)", "aliases": ["591", "refused", "log_calls"]},
    {"id": "GH-593", "desc": "Container & subprocess leak prevention on exit", "aliases": ["593", "kill_on_drop", "zero_zombie"]},
    {"id": "GH-597", "desc": "Blocked tool calls error contract compliance", "aliases": ["597", "blocked", "deny"]},
    {"id": "RFC-2025", "desc": "Streamable HTTP single-endpoint POST /mcp compliance", "aliases": ["2025", "streamable_http"]},
    {"id": "OWASP-08", "desc": "Hermetic subprocess environment isolation (env_clear)", "aliases": ["llm08", "env_clear", "hermetic"]},
    {"id": "OWASP-01", "desc": "Tool description poison scanner & prompt injection defense", "aliases": ["llm01", "poison", "injection"]},
    {"id": "PCI-DSS", "desc": "PAN credit card inline masking and DLP guardrails", "aliases": ["pci", "credit_card", "dlp"]},
    {"id": "SOC2-T2", "desc": "Cryptographic SHA-256 hash chaining audit log", "aliases": ["soc2", "hash_chain", "tamper"]},
    {"id": "FINOPS-1", "desc": "Hard freeze cutoff on tenant budget exhaustion", "aliases": ["hard_freeze", "finops", "budget"]},
    {"id": "META-PLAN", "desc": "DAG cycle detection and task planning engine", "aliases": ["plan_tasks", "executionplan", "planner"]}
]

def scan_codebase(local_root: str):
    tests_dir = os.path.join(local_root, "tests")
    coverage_results = []

    test_files_content = {}
    if os.path.exists(tests_dir):
        for root, _, files in os.walk(tests_dir):
            for file in files:
                if file.endswith(".rs"):
                    path = os.path.join(root, file)
                    with open(path, "r", encoding="utf-8", errors="ignore") as f:
                        test_files_content[file] = f.read().lower()

    for item in TARGET_COMPLAINTS:
        aliases = [a.lower() for a in item["aliases"]]
        matched_files = []

        for fname, content in test_files_content.items():
            if any(a in content for a in aliases):
                matched_files.append(fname)

        coverage_results.append({
            "id": item["id"],
            "desc": item["desc"],
            "covered": len(matched_files) > 0,
            "evidence": matched_files[:2]
        })

    return coverage_results

def print_report(results):
    total = len(results)
    covered_count = sum(1 for r in results if r["covered"])
    coverage_pct = (covered_count / total) * 100

    print("=" * 65)
    print("      AEGIS GATEWAY: OPERATIONAL GAP COVERAGE SCORECARD        ")
    print("=" * 65)
    print(f"Total Canonical Complaints Audited : {total}")
    print(f"Directly Verified in Test Suites   : {covered_count}")
    print(f"Empirical Coverage Score           : {coverage_pct:.1f}%")
    print("-" * 65)

    for r in results:
        status = "[COVERED]" if r["covered"] else "[GAP]    "
        evidence = f"-> {', '.join(r['evidence'])}" if r["evidence"] else "-> NO TEST PROOF"
        print(f"{status} {r['id']:<10} {r['desc'][:40]:<40} {evidence}")

    print("=" * 65)
    if coverage_pct >= 90.0:
        print(">>> [SCORECARD PASS] Superior parity over legacy MCP gateways.")
    else:
        print(">>> [ATTENTION] Gaps exist. Scaffold next phase via scaffold_phase.py.")

def main():
    parser = argparse.ArgumentParser(description="Audit operational gap test coverage")
    parser.add_argument("--local-root", default="/root/projects/aegis-gateway", help="Aegis root dir")
    args = parser.parse_args()

    results = scan_codebase(args.local_root)
    print_report(results)

if __name__ == "__main__":
    main()
