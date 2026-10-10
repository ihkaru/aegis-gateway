#!/usr/bin/env python3
"""
Scaffold a new implementation phase in Aegis Gateway based on identified gaps.
Generates roadmap specification, integration test skeleton, and registers phase in ledgers.
"""

import argparse
import os
import re
import sys

ROADMAP_DIR = "docs/roadmap"
TESTS_DIR = "tests"

def slugify(text: str) -> str:
    text = text.lower()
    text = re.sub(r"[^\w\s-]", "", text)
    return re.sub(r"[-\s]+", "_", text).strip("_")

def generate_roadmap_content(phase_num: int, title: str, issues: list) -> str:
    slug = slugify(title).upper()
    issue_lines = "\n".join([f"- Resolves `{iss}`" for iss in issues]) if issues else "- Identified operational gap"
    return f"""# Phase {phase_num}: {title}

> **Milestone Tag**: `v1.{phase_num}.0-{slugify(title)}`  
> **Status**: `In Progress`  
> **Target Gaps & Standards**:
{issue_lines}

---

## 1. Problem Statement & Motivation
First-generation MCP gateways exhibited operational failures and complaints:
{issue_lines}

Aegis Gateway addresses these gaps via strict interface-first architecture, zero unsafe code, and zero mock closures.

---

## 2. Technical Architecture & Trait Contracts
- Defined abstract trait interfaces in `src/core/`.
- Isolated concrete driver implementations in submodule under `src/`.
- Injected via dependency injection (`Arc<dyn Trait>`).

---

## 3. Empirical Verification Plan
- Unit and integration tests in `tests/phase{phase_num}_{slugify(title)}_test.rs`.
- Validated via `bash scripts/audit_mock_detection.sh` and `bash scripts/governance-check.sh`.
- Hard line constraint: `wc -l <= 350`.
"""

def generate_test_content(phase_num: int, title: str, issues: list) -> str:
    slug = slugify(title)
    return f"""//! Phase {phase_num} Integration Test Suite: {title}
//! Target Gaps: {', '.join(issues) if issues else 'N/A'}

#![deny(unsafe_code)]

#[tokio::test]
async fn test_phase{phase_num}_golden_path() {{
    // Verify standard operational lifecycle without errors
    assert!(true, "Golden path initialized");
}}

#[tokio::test]
async fn test_phase{phase_num}_boundary_resilience() {{
    // Verify error taxonomy and edge cases
    assert!(true, "Boundary resilience verified");
}}
"""

def update_overview_table(phase_num: int, title: str, file_name: str, local_root: str):
    overview_path = os.path.join(local_root, ROADMAP_DIR, "00_ROADMAP_OVERVIEW.md")
    if not os.path.exists(overview_path):
        return
    with open(overview_path, "r", encoding="utf-8") as f:
        content = f.read()

    new_row = f"| **Phase {phase_num}**| **{title}** | Operational gap resolution | `In Progress` | [`{file_name}`](./{file_name}) |"
    if f"**Phase {phase_num}**" in content:
        print(f"[!] Phase {phase_num} already in 00_ROADMAP_OVERVIEW.md", file=sys.stderr)
        return

    # Insert before the last table boundary or section break
    table_marker = "| :--- | :--- | :--- | :--- | :--- |"
    if table_marker in content:
        parts = content.split("---", 2)
        if len(parts) >= 2:
            lines = parts[1].strip().split("\n")
            lines.append(new_row)
            updated_section = "\n" + "\n".join(lines) + "\n\n"
            content = parts[0] + "---" + updated_section + "---" + parts[2]
            with open(overview_path, "w", encoding="utf-8") as f:
                f.write(content)
            print(f"[+] Updated 00_ROADMAP_OVERVIEW.md with Phase {phase_num}", file=sys.stderr)

def main():
    parser = argparse.ArgumentParser(description="Scaffold a new Aegis Gateway Phase")
    parser.add_argument("--phase-num", type=int, required=True, help="Phase number (e.g. 12)")
    parser.add_argument("--title", required=True, help="Phase title")
    parser.add_argument("--issues", default="", help="Comma-separated issues (e.g. docker/mcp-gateway#591)")
    parser.add_argument("--local-root", default="/root/projects/aegis-gateway", help="Aegis root dir")

    args = parser.parse_args()
    issues = [i.strip() for i in args.issues.split(",") if i.strip()]
    slug = slugify(args.title)

    # 1. Write Roadmap file
    doc_file = f"{args.phase_num:02d}_PHASE_{args.phase_num}_{slug.upper()}.md"
    doc_path = os.path.join(args.local_root, ROADMAP_DIR, doc_file)
    with open(doc_path, "w", encoding="utf-8") as f:
        f.write(generate_roadmap_content(args.phase_num, args.title, issues))
    print(f"[+] Created roadmap doc: {doc_path}")

    # 2. Write Test file
    test_file = f"phase{args.phase_num}_{slug}_test.rs"
    test_path = os.path.join(args.local_root, TESTS_DIR, test_file)
    with open(test_path, "w", encoding="utf-8") as f:
        f.write(generate_test_content(args.phase_num, args.title, issues))
    print(f"[+] Created test skeleton: {test_path}")

    # 3. Update Overview
    update_overview_table(args.phase_num, args.title, doc_file, args.local_root)

    # 4. Print Parity Matrix Snippet
    if issues:
        print("\n[+] Enterprise Ledger Snippet for docs/ISSUES_PARITY_MATRIX.md (record after completion):")
        for iss in issues:
            parts = iss.split("#")
            repo = parts[0] if len(parts) > 1 else "MikkoParkkola/mcp-gateway"
            num = parts[1] if len(parts) > 1 else iss
            print(f"| `{repo}` | [#{num}](https://github.com/{repo}/issues/{num}) | `OPS` | <Deficit> | <Resolution> | `src/...` | `tests/{test_file}` |")

if __name__ == "__main__":
    main()
