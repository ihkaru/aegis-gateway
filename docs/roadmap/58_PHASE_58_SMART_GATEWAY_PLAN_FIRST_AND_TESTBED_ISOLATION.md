# Phase 58: Smart Gateway Architecture, Testbed Stub Isolation, and Plan-First Doctrine

## 1. Executive Summary & Problem Formulation
In previous iterations, the gateway seeded 18 synthetic testbed tool definitions (`stripe_*`, `query_postgresql_ledger`, `destructive_drop_table`, `tax_calculate_vat`, etc.) into the active runtime catalog whenever the gateway started without an explicit `propylon.yaml`. These synthetic tools were originally created as fixture testbeds for DLP and ABAC policy evaluation, but when exposed to live MCP clients (Claude Code, Cursor, Antigravity), they created significant context bloat, token waste, and confusion. Furthermore, redundant aliases (`gateway_execute_code` alongside `execute_code`) and lack of a structured planning gate led to disjointed tool executions.

Phase 58 implements enterprise best practices by establishing:
1. **Strict Testbed Stub Isolation**: 100% of synthetic/mock tool definitions are removed from production default catalogs. The gateway boots in Clean Gateway Mode with zero synthetic fixtures unless `PROPYLON_ENABLE_DEMO_FIXTURES=true` is explicitly configured.
2. **Redundancy Elimination**: Duplicate tool aliases are deduplicated from the advertised wire catalog (retaining transparent backward compatibility in execution dispatchers).
3. **Plan-First Doctrine (Prinsip Masuk Akal Dulu)**: Elevating `gateway_plan_tasks` into a first-class cognitive entrypoint where complex or multi-step tasks are decomposed, validated against ABAC security policies, and preflighted before execution.
4. **Progressive Tool Discovery**: Exposing only high-leverage meta-tools upfront (7 tools total), allowing agents to search and bind external tools on-demand via `gateway_search_tools`.

## 2. Architecture & Invariants
- **Clean Production Wire Surface**: Only authentic native gateway engines (`gateway_plan_tasks`, `gateway_search_tools`, `execute_code`, `execute_in_situ_query`, `resolve_approval`, `gateway_list_servers`, `gateway_register_tools`) plus genuinely attached backends are advertised.
- **Natural Language Goal Planning**: `gateway_plan_tasks` accepts natural language `goal` in Indonesian and English, outputting structured DAG execution plans with risk scoring and approval requirements.
- **Strict File Limit**: All modified and new files strictly adhere to `wc -l <= 350`.
- **Pure-Rust & Zero-Mock**: `#![deny(unsafe_code)]` preserved, and zero mock execution in production pathways.
