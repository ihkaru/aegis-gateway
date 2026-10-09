# Phase 6: Centralized Skill OS & Progressive Disclosure

> **Milestone Tag**: `v0.7.0-skill-os`  
> **Status**: `In Progress` (SkillRegistry contract and Local loader verified; GitOps sync & Semantic RAG planned)

---

## 1. Objectives

Transform Aegis Gateway into an enterprise-wide Skill Operating System. Instead of bloating agent context windows with thousands of static system prompt lines or duplicating skill scripts across individual developer machines, Aegis provides a centralized, versioned, hot-reloadable skill catalog with on-demand progressive activation.

---

## 2. Architecture & Contracts

Defined in [`src/core/skills.rs`](../../src/core/skills.rs):

```rust
#[async_trait]
pub trait SkillRegistry: Send + Sync {
    async fn list_skills(&self) -> AegisResult<Vec<SkillMetadata>>;
    async fn load_skill(&self, name: &str) -> AegisResult<SkillBundle>;
    async fn reload(&self) -> AegisResult<usize>;
}
```

### Progressive Skill Lifecycle

```mermaid
sequenceDiagram
    participant Agent as Coding Agent
    participant Gateway as Aegis Gateway
    participant Registry as Central Skill Registry
    participant Git as Enterprise Git Repository

    Git->>Registry: Webhook Push (Skill GitOps Update)
    Registry->>Gateway: Hot-Reload Registry (Zero Downtime)
    Agent->>Gateway: list_skills() [Tier 1: Name + Summary Only (~50 tokens)]
    Gateway-->>Agent: Returns Skill Metadata
    Agent->>Gateway: load_skill("security-auditor") [Tier 2: On-Demand Full Prompt]
    Gateway-->>Agent: Returns Full Markdown Instructions & Auxiliary Scripts
```

1. **Tier 1 (Catalog Indexing)**:
   - Returns only lightweight `SkillMetadata` (name, short description, category tags).
   - Minimal token footprint; enables indexing hundreds of enterprise skills simultaneously.
2. **Tier 2 (On-Demand Activation)**:
   - Full instructions, SOP procedures, and executable scripts (`SkillBundle`) are fetched only when the agent decides to execute that specific workflow.
3. **Enterprise Skill Provenance**:
   - Every skill bundle is signed cryptographically by the organization's security team to prevent supply-chain poisoning.

---

## 3. Milestones & Checklist

- [x] **6.1 SkillRegistry Trait Abstraction**: Abstract metadata indexing and full bundle loading into `src/core/skills.rs` (Empirically verified in `tests/enterprise_governance_test.rs`).
- [x] **6.2 Local Skill Registry Loader**: Implement `LocalSkillRegistry` reading `.agents/skills` directories and parsing `SKILL.md` frontmatter.
- [x] **6.3 Agent Governance Skills Suite**: Ship built-in enterprise audit skills:
  - `enterprise-readiness-auditor`
  - `mcp-protocol-governor`
  - `solid-code-reviewer`
- [ ] **6.4 GitOps Remote Skill Synchronization**: Auto-clone and pull enterprise skill repositories via SSH/HTTPS with GPG/Cosign signature validation.
- [ ] **6.5 Semantic Skill Vector Retrieval (RAG)**: Pure-Rust SIMD HNSW vector search to recommend skills based on the user's immediate prompt without prompt-stuffing.
- [ ] **6.6 Skill Compatibility & SemVer Engine**: Enforce minimum agent runtime versions and declare prerequisite MCP tool dependencies.
