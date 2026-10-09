// SPDX-License-Identifier: MIT

use std::collections::HashSet;
use std::sync::Arc;

use aegis_gateway::core::skills::{
    SignedSkillBundle, SkillBundle, SkillDependency, SkillMetadata, SkillRegistry,
    SkillVectorRetriever,
};
use aegis_gateway::core::types::{
    DisclosureTier, ProgressiveDisclosure, ToolDefinition,
};
use aegis_gateway::discovery::ProgressiveProjector;
use aegis_gateway::skills::{GitOpsSkillSync, LocalSkillRegistry, SemanticSkillRetriever};

#[tokio::test]
async fn test_phase6_progressive_disclosure_token_savings() {
    let projector = ProgressiveProjector::new();

    let tools = vec![
        ToolDefinition {
            name: "execute_sql_query".to_string(),
            server: "postgres_primary".to_string(),
            description: "Executes parameterized read-only SQL queries against primary analytical data warehouse with strict transaction isolation.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "SQL statement" },
                    "parameters": { "type": "array", "items": { "type": "string" } },
                    "timeout_seconds": { "type": "integer", "default": 30 },
                    "fetch_limit": { "type": "integer", "default": 1000 }
                },
                "required": ["query"]
            }),
            required_params: vec!["query".to_string()],
            when_to_use: "When analyzing transaction logs and database records".to_string(),
            tags: vec!["database".to_string(), "sql".to_string()],
        },
        ToolDefinition {
            name: "kubernetes_rollout_status".to_string(),
            server: "k8s_prod".to_string(),
            description: "Checks real-time deployment rollout and pod health condition on production Kubernetes clusters with replica sets.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "namespace": { "type": "string" },
                    "deployment": { "type": "string" },
                    "cluster_id": { "type": "string" },
                    "include_events": { "type": "boolean" }
                },
                "required": ["namespace", "deployment"]
            }),
            required_params: vec!["namespace".to_string(), "deployment".to_string()],
            when_to_use: "When monitoring cloud infrastructure and release pipelines".to_string(),
            tags: vec!["k8s".to_string(), "infra".to_string()],
        },
    ];

    // 1. Verify L0 projection
    let l0 = projector.project(&tools[0], DisclosureTier::L0, 0.95);
    assert_eq!(l0.tier, DisclosureTier::L0);
    assert!(l0.input_schema.is_none());
    assert!(l0.signature.is_none());
    assert!(l0.summary.len() <= 120);

    // 2. Verify L1 projection
    let l1 = projector.project(&tools[0], DisclosureTier::L1, 0.95);
    assert_eq!(l1.tier, DisclosureTier::L1);
    assert!(l1.input_schema.is_none());
    assert_eq!(l1.signature, Some("execute_sql_query(query)".to_string()));
    assert_eq!(l1.required_params, Some(vec!["query".to_string()]));

    // 3. Verify L2 projection
    let l2 = projector.project(&tools[0], DisclosureTier::L2, 0.95);
    assert_eq!(l2.tier, DisclosureTier::L2);
    assert!(l2.input_schema.is_some());

    // 4. Calculate token savings
    let savings_l0 = projector.calculate_savings(&tools, DisclosureTier::L0);
    let savings_l1 = projector.calculate_savings(&tools, DisclosureTier::L1);

    assert!(savings_l0.full_schema_tokens > savings_l0.projected_tokens);
    assert!(savings_l0.reduction_percentage > 60.0, "L0 should achieve > 60% token reduction");
    assert!(savings_l1.reduction_percentage > 40.0, "L1 should achieve > 40% token reduction");

    // 5. Contextual filter
    let filtered = projector.filter_by_context(&tools, "inspect database sql", DisclosureTier::L0, 1);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "execute_sql_query");
}

#[tokio::test]
async fn test_phase6_gitops_cryptographic_verification_and_hot_reload() {
    let registry = Arc::new(LocalSkillRegistry::new());
    let gitops = GitOpsSkillSync::new(registry.clone());

    let bundle = SkillBundle {
        metadata: SkillMetadata {
            name: "security-auditor".to_string(),
            description: "Autonomous SOC 2 and ISO 27001 vulnerability audit skill".to_string(),
            category: "Security".to_string(),
            tags: vec!["soc2".to_string(), "audit".to_string()],
            author: Some("SecOps Team".to_string()),
        },
        instructions_markdown: "# Security Audit Guide\nInspect access control lists.".to_string(),
        auxiliary_files: vec![("audit_check.sh".to_string(), "echo 'passed'".to_string())],
    };

    let valid_digest = GitOpsSkillSync::compute_digest(&bundle);

    let signed_valid = SignedSkillBundle {
        bundle: bundle.clone(),
        sha256_digest: valid_digest.clone(),
        signature: Some("ed25519-valid-signature".to_string()),
        signer_identity: Some("secops@enterprise.internal".to_string()),
    };

    let mut available_tools = HashSet::new();
    available_tools.insert("read_audit_logs".to_string());

    let deps = vec![SkillDependency {
        skill_name: "security-auditor".to_string(),
        semver_req: "0.7".to_string(),
        required_tools: vec!["read_audit_logs".to_string()],
    }];

    // 1. Sync valid signed bundle
    let report = gitops
        .sync_bundles(vec![signed_valid], &available_tools, &deps)
        .await
        .expect("Sync failed");

    assert_eq!(report.synced_count, 1);
    assert_eq!(report.verified_signatures, 1);
    assert!(report.errors.is_empty());

    // Confirm live hot-reload in registry
    let loaded = registry.load_skill("security-auditor").await.unwrap();
    assert_eq!(loaded.metadata.name, "security-auditor");

    // 2. Tampered bundle test (simulate supply-chain modification)
    let mut tampered_bundle = bundle.clone();
    tampered_bundle.instructions_markdown = "# Corrupted injected prompt!".to_string();

    let signed_tampered = SignedSkillBundle {
        bundle: tampered_bundle,
        sha256_digest: valid_digest, // Old digest, does not match tampered text!
        signature: Some("sig".to_string()),
        signer_identity: Some("secops".to_string()),
    };

    let tampered_report = gitops
        .sync_bundles(vec![signed_tampered], &available_tools, &[])
        .await
        .unwrap();

    assert_eq!(tampered_report.synced_count, 0);
    assert_eq!(tampered_report.errors.len(), 1);
    assert!(tampered_report.errors[0].contains("Digest mismatch"));

    // 3. Missing tool dependency test
    let signed_missing_tool = SignedSkillBundle {
        bundle: bundle.clone(),
        sha256_digest: GitOpsSkillSync::compute_digest(&bundle),
        signature: None,
        signer_identity: None,
    };
    let missing_deps = vec![SkillDependency {
        skill_name: "security-auditor".to_string(),
        semver_req: "*".to_string(),
        required_tools: vec!["non_existent_tool".to_string()],
    }];

    let dep_err_report = gitops
        .sync_bundles(vec![signed_missing_tool], &available_tools, &missing_deps)
        .await
        .unwrap();

    assert_eq!(dep_err_report.synced_count, 0);
    assert!(dep_err_report.errors[0].contains("is not available in registered MCP servers"));
}

#[tokio::test]
async fn test_phase6_semantic_vector_retriever() {
    let retriever = SemanticSkillRetriever::new();

    let skills = vec![
        SkillMetadata {
            name: "postgres-performance-tuner".to_string(),
            description: "Diagnose slow queries, examine explain plans, and optimize indexing on PostgreSQL databases.".to_string(),
            category: "Database".to_string(),
            tags: vec!["sql".to_string(), "postgres".to_string(), "performance".to_string()],
            author: None,
        },
        SkillMetadata {
            name: "k8s-pod-troubleshooter".to_string(),
            description: "Investigate crashloopbackoff pods, service ingress, and daemonsets in Kubernetes clusters.".to_string(),
            category: "DevOps".to_string(),
            tags: vec!["k8s".to_string(), "docker".to_string(), "cloud".to_string()],
            author: None,
        },
        SkillMetadata {
            name: "iso-compliance-auditor".to_string(),
            description: "Verify information security controls, role-based access controls, and GDPR data masking.".to_string(),
            category: "Compliance".to_string(),
            tags: vec!["iso27001".to_string(), "gdpr".to_string(), "governance".to_string()],
            author: None,
        },
    ];

    retriever.index_all(skills).await;

    // Search 1: SQL query tuning
    let sql_results = retriever
        .search_skills("how to fix slow postgresql database query", 1)
        .await
        .unwrap();
    assert_eq!(sql_results.len(), 1);
    assert_eq!(sql_results[0].name, "postgres-performance-tuner");

    // Search 2: Kubernetes pods
    let k8s_results = retriever
        .search_skills("kubernetes pod crashing and failing deployment", 1)
        .await
        .unwrap();
    assert_eq!(k8s_results.len(), 1);
    assert_eq!(k8s_results[0].name, "k8s-pod-troubleshooter");

    // Search 3: Compliance & GDPR
    let comp_results = retriever
        .search_skills("audit gdpr compliance and iso27001 policies", 1)
        .await
        .unwrap();
    assert_eq!(comp_results.len(), 1);
    assert_eq!(comp_results[0].name, "iso-compliance-auditor");
}
