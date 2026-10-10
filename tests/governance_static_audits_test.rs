// SPDX-License-Identifier: MIT

use std::process::Command;

#[test]
fn test_governance_zero_mock_integrity() {
    let output = Command::new("bash")
        .arg("scripts/audit_mock_detection.sh")
        .output()
        .expect("Failed to execute zero-mock audit");

    assert!(
        output.status.success(),
        "Zero-mock audit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_governance_solid_and_350_line_limit() {
    let output = Command::new("bash")
        .arg(".agents/skills/solid-code-reviewer/scripts/audit_solid.sh")
        .output()
        .expect("Failed to execute SOLID audit");

    assert!(
        output.status.success(),
        "SOLID & 350-line audit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_governance_enterprise_readiness_pillars() {
    let output = Command::new("bash")
        .arg(".agents/skills/enterprise-readiness-auditor/scripts/audit_enterprise.sh")
        .output()
        .expect("Failed to execute enterprise readiness audit");

    assert!(
        output.status.success(),
        "Enterprise readiness audit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_governance_mcp_protocol_standards() {
    let output = Command::new("bash")
        .arg(".agents/skills/mcp-protocol-governor/scripts/audit_mcp.sh")
        .output()
        .expect("Failed to execute MCP protocol audit");

    assert!(
        output.status.success(),
        "MCP protocol audit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_governance_legacy_mcp_gap_resolution() {
    let output = Command::new("bash")
        .arg(".agents/skills/mcp-enterprise-gap-auditor/scripts/audit_mcp_gaps.sh")
        .output()
        .expect("Failed to execute legacy MCP gap audit");

    assert!(
        output.status.success(),
        "Legacy MCP gap audit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_governance_zero_downtime_invariants() {
    let output = Command::new("bash")
        .arg(".agents/skills/zero-downtime-control-plane-auditor/scripts/audit_zero_downtime.sh")
        .output()
        .expect("Failed to execute zero-downtime audit");

    assert!(
        output.status.success(),
        "Zero downtime audit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
