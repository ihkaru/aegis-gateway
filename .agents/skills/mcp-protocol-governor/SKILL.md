---
name: mcp-protocol-governor
description: Protocol and security compliance auditor for MCP (Model Context Protocol), Progressive Disclosure (L0/L1/L2), Tool Poisoning defense, and Centralized Skill Bundling.
---

# MCP Protocol Governor Skill

This skill enforces strict Model Context Protocol (MCP) conformance, anti-poisoning defenses, and skill lifecycle integrity.

## Key Compliance Requirements

1. **Protocol Conformance (JSON-RPC 2.0)**
   - Strict adherence to JSON-RPC 2.0 specification (`id`, `jsonrpc: "2.0"`, `method`, `params`).
   - Standard MCP methods: `initialize`, `tools/list`, `tools/call`, `prompts/list`, `prompts/get`, `resources/list`, `resources/read`.

2. **Progressive Disclosure (Context Optimization)**
   - **Tier L0 (Discovery)**: Tool name, one-line purpose (<=120 chars), relevance score.
   - **Tier L1 (Selection)**: L0 + functional signature, required parameters, and `when_to_use` guidance (<=280 chars).
   - **Tier L2 (Execution)**: Complete JSON Schema definition for tool inputs.
   - Prevents context window exhaustion when thousands of tools are registered.

3. **Active Tool Poisoning & Prompt Injection Defense**
   - Pre-ingestion scanning of tool and skill descriptions.
   - Detection of hidden prompt injection blocks: `<IMPORTANT>`, `<system>`, base64 exfiltration triggers, sensitive path probing (`/etc/passwd`, `~/.ssh`, `id_rsa`, `.env`).
   - SHA-256 capability pinning and rug-pull quarantine.

4. **Skill Registry & Hot-Reload Integrity**
   - Autonomous extraction and generation of `SKILL.md` bundles with YAML frontmatter.
   - Asynchronous file watching for live hot-reloading without agent restarts.
   - Automatic injection into `.agents/skills/` and `.claude/skills/`.

## Enforcement Procedure

Run the MCP protocol audit script:
```bash
bash .agents/skills/mcp-protocol-governor/scripts/audit_mcp.sh
```
All assertions must pass before committing any protocol-level modifications.
