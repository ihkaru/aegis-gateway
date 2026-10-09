#!/usr/bin/env python3
"""
Research GitHub Gaps for MCP Gateways.
Queries open & closed issues from reference MCP gateway repositories,
categorizes operational failure modes, and cross-references against local Aegis code.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import urllib.request
from typing import Dict, List, Any

TAXONOMY_RULES = {
    "SECURITY_GOVERNANCE": [
        "policy", "auth", "abac", "rbac", "token", "leak", "secret", "inject",
        "refuse", "blocked", "permission", "credential", "attestation", "pci", "dlp"
    ],
    "SUBPROCESS_LIFECYCLE": [
        "zombie", "process", "container", "kill", "sigterm", "leak", "spawn",
        "orphan", "exit", "crash", "subprocess", "stdio", "hang", "pipe"
    ],
    "PROTOCOL_CONFORMANCE": [
        "jsonrpc", "protocol", "header", "version", "sse", "stream", "http",
        "handshake", "ping", "initialize", "schema", "payload", "error"
    ],
    "OPERATIONAL_RELIABILITY": [
        "cache", "timeout", "reconnect", "retry", "circuit", "fail", "slow",
        "disconnect", "deadlock", "drain", "memory", "buffer", "overflow"
    ],
    "DISCOVERY_PLANNING": [
        "search", "plan", "filter", "list", "drift", "definition", "discovery",
        "prompt", "token", "progressive", "dag", "catalog"
    ]
}

def fetch_issues_gh(repo: str, state: str, limit: int, query: str = "") -> List[Dict[str, Any]]:
    cmd = [
        "gh", "issue", "list",
        "--repo", repo,
        "--state", state,
        "--limit", str(limit),
        "--json", "number,title,state,url,updatedAt,labels,body"
    ]
    if query:
        cmd.extend(["--search", query])
    try:
        res = subprocess.run(cmd, capture_output=True, text=True, check=True)
        return json.loads(res.stdout)
    except Exception as e:
        print(f"[WARN] gh CLI failed for {repo}: {e}. Falling back to REST API...", file=sys.stderr)
        return fetch_issues_rest(repo, state, limit)

def fetch_issues_rest(repo: str, state: str, limit: int) -> List[Dict[str, Any]]:
    url = f"https://api.github.com/repos/{repo}/issues?state={state}&per_page={min(limit, 100)}"
    req = urllib.request.Request(url, headers={"User-Agent": "Aegis-Gap-Research/1.0"})
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return [
                {
                    "number": item.get("number"),
                    "title": item.get("title"),
                    "state": item.get("state"),
                    "url": item.get("html_url"),
                    "updatedAt": item.get("updated_at"),
                    "labels": [l.get("name") for l in item.get("labels", [])],
                    "body": item.get("body", "") or ""
                }
                for item in data if "pull_request" not in item
            ]
    except Exception as err:
        print(f"[ERROR] REST API failed for {repo}: {err}", file=sys.stderr)
        return []

def classify_issue(title: str, body: str) -> str:
    content = f"{title} {body}".lower()
    for category, keywords in TAXONOMY_RULES.items():
        if any(kw in content for kw in keywords):
            return category
    return "GENERAL_OPERATIONAL"

def check_local_coverage(issue_num: int, title: str, local_root: str) -> Dict[str, Any]:
    keywords = [f"#{issue_num}", f"issue_{issue_num}", f"issue {issue_num}"]
    # Extract distinct keywords from title
    clean_words = re.findall(r"\b[a-zA-Z]{5,}\b", title.lower())
    title_sub = clean_words[:2] if clean_words else []

    tests_dir = os.path.join(local_root, "tests")
    src_dir = os.path.join(local_root, "src")

    match_files = []
    for search_dir in [tests_dir, src_dir]:
        if not os.path.exists(search_dir):
            continue
        for root, _, files in os.walk(search_dir):
            for file in files:
                if not file.endswith(".rs"):
                    continue
                path = os.path.join(root, file)
                try:
                    with open(path, "r", encoding="utf-8", errors="ignore") as f:
                        text = f.read().lower()
                    if any(kw.lower() in text for kw in keywords):
                        match_files.append((file, "explicit_issue_ref"))
                    elif title_sub and all(w in text for w in title_sub):
                        match_files.append((file, "semantic_match"))
                except Exception:
                    pass

    if match_files:
        return {
            "covered": True,
            "evidence": [f"{f} ({r})" for f, r in match_files[:2]]
        }
    return {"covered": False, "evidence": []}

def calculate_priority(category: str, state: str, covered: bool) -> str:
    if covered:
        return "RESOLVED"
    if category in ["SECURITY_GOVERNANCE", "SUBPROCESS_LIFECYCLE"]:
        return "P0 (Critical)"
    if category in ["PROTOCOL_CONFORMANCE", "OPERATIONAL_RELIABILITY"]:
        return "P1 (High)"
    return "P2 (Medium)"

def generate_report(results: List[Dict[str, Any]], repos: List[str], format_type: str) -> str:
    if format_type == "json":
        return json.dumps(results, indent=2)

    total = len(results)
    unresolved = [r for r in results if not r["coverage"]["covered"]]
    resolved = total - len(unresolved)

    lines = [
        "# MCP Gateway Comparative Gap Analysis Report",
        "",
        f"> **Analyzed Repositories**: {', '.join(repos)}  ",
        f"> **Total Target Issues**: {total} | **Covered in Aegis**: {resolved} | **Unresolved Gaps**: {len(unresolved)}  ",
        "",
        "## 1. High-Priority Unresolved Gaps (Actionable for Next Phase)",
        "",
        "| Repo | Issue | State | Category | Priority | Problem Summary |",
        "| :--- | :--- | :--- | :--- | :--- | :--- |"
    ]

    for item in sorted(unresolved, key=lambda x: x["priority"]):
        title_esc = item["title"].replace("|", "\\|")
        lines.append(
            f"| `{item['repo']}` | [#{item['number']}]({item['url']}) | `{item['state']}` | "
            f"`{item['category']}` | **{item['priority']}** | {title_esc[:80]} |"
        )

    lines.extend([
        "",
        "## 2. Already Resolved Gaps & Parity Proof",
        "",
        "| Repo | Issue | Category | Aegis Verification Evidence |",
        "| :--- | :--- | :--- | :--- |"
    ])

    for item in [r for r in results if r["coverage"]["covered"]][:10]:
        evidence_str = ", ".join(item["coverage"]["evidence"])
        lines.append(
            f"| `{item['repo']}` | [#{item['number']}]({item['url']}) | "
            f"`{item['category']}` | `{evidence_str}` |"
        )

    return "\n".join(lines)

def main():
    parser = argparse.ArgumentParser(description="Research gaps from legacy MCP gateways")
    parser.add_argument("--repo", default="docker/mcp-gateway,microsoft/mcp-gateway",
                        help="Comma-separated list of GitHub repositories")
    parser.add_argument("--state", default="all", choices=["open", "closed", "all"],
                        help="Issue state to query")
    parser.add_argument("--limit", type=int, default=20,
                        help="Max issues to fetch per repository")
    parser.add_argument("--query", default="",
                        help="Optional search query filter")
    parser.add_argument("--local-root", default="/root/projects/aegis-gateway",
                        help="Path to local Aegis Gateway repository")
    parser.add_argument("--format", default="markdown", choices=["markdown", "json"],
                        help="Output format")
    parser.add_argument("--output", default="",
                        help="Output file path (default stdout)")

    args = parser.parse_args()
    repos = [r.strip() for r in args.repo.split(",") if r.strip()]

    all_results = []
    for repo in repos:
        print(f"[*] Querying issues from '{repo}' (state={args.state}, limit={args.limit})...", file=sys.stderr)
        issues = fetch_issues_gh(repo, args.state, args.limit, args.query)
        for issue in issues:
            category = classify_issue(issue.get("title", ""), issue.get("body", ""))
            coverage = check_local_coverage(issue.get("number", 0), issue.get("title", ""), args.local_root)
            priority = calculate_priority(category, issue.get("state", "open"), coverage["covered"])
            all_results.append({
                "repo": repo,
                "number": issue.get("number"),
                "title": issue.get("title"),
                "url": issue.get("url"),
                "state": issue.get("state"),
                "category": category,
                "coverage": coverage,
                "priority": priority,
                "body_preview": (issue.get("body", "") or "")[:200]
            })

    output_content = generate_report(all_results, repos, args.format)
    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(output_content)
        print(f"[+] Analysis report written to {args.output}", file=sys.stderr)
    else:
        print(output_content)

if __name__ == "__main__":
    main()
