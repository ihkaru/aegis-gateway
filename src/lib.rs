// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

//! # Aegis Gateway
//!
//! Enterprise Zero-Trust MCP & Skill Gateway with Distributed State,
//! Granular ABAC, Real-time DLP, and Tamper-Proof Audit.

pub mod audit;
pub mod backend;
pub mod cli;
pub mod core;
pub mod daemon;
pub mod discovery;
pub mod dlp;
pub mod k8s;
pub mod policy;
pub mod sandbox;
pub mod skills;
pub mod state;
pub mod transport;
pub mod trust;
pub mod vault;

use std::sync::Arc;

use crate::core::approval::ApprovalGate;
use crate::core::audit::AuditSink;
use crate::core::data_governance::{DataEgressPolicyEngine, InSituDataEnclave, PolicyTier};
use crate::core::delegation::IdentityDelegationBroker;
use crate::core::dlp::DlpPipeline;
use crate::core::error::AegisResult;
use crate::core::notification::{
    ApprovalChannelTarget, ApprovalNotificationDispatcher, DurableResumeRouter,
};
use crate::core::oauth_connect::OAuthConnectEngine;
use crate::core::policy::PolicyEngine;
use crate::core::proxy::CredentialProxyEngine;
use crate::core::sandbox::CodeSandboxEngine;
use crate::core::secrets::SecretStore;
use crate::core::skills::SkillRegistry;
use crate::core::state::DistributedState;
use crate::core::types::{
    DisclosureTier, ProjectedTool, ToolCallRequest, ToolCallResponse, ToolDefinition,
};
use crate::transport::run_tool_execution_pipeline;

/// High-level Enterprise Gateway Orchestrator (Dependency Inversion applied)
pub struct AegisGateway {
    state: Arc<dyn DistributedState>,
    policy: Arc<dyn PolicyEngine>,
    dlp: Arc<dyn DlpPipeline>,
    audit: Arc<dyn AuditSink>,
    skills: Arc<dyn SkillRegistry>,
    sandbox: Arc<dyn CodeSandboxEngine>,
    drain: Arc<state::DrainCoordinator>,
    secret_store: Arc<dyn SecretStore>,
    proxy: Arc<dyn CredentialProxyEngine>,
    approval: Arc<dyn ApprovalGate>,
    delegation: Arc<dyn IdentityDelegationBroker>,
    oauth_connect: Arc<dyn OAuthConnectEngine>,
    data_egress: Arc<dyn DataEgressPolicyEngine>,
    in_situ: Arc<dyn InSituDataEnclave>,
    approval_dispatcher: Arc<dyn ApprovalNotificationDispatcher>,
    resume_router: Arc<dyn DurableResumeRouter>,
}

impl AegisGateway {
    pub fn new(
        state: Arc<dyn DistributedState>,
        policy: Arc<dyn PolicyEngine>,
        dlp: Arc<dyn DlpPipeline>,
        audit: Arc<dyn AuditSink>,
        skills: Arc<dyn SkillRegistry>,
    ) -> Self {
        let egress = Arc::new(sandbox::EgressFilterEngine::new());
        let secret_store: Arc<dyn SecretStore> = Arc::new(policy::secrets::EnvSecretStore::new());
        let broker = Arc::new(sandbox::VaultCredentialBroker::new(secret_store.clone()));
        let default_sandbox = Arc::new(sandbox::HermeticProcessSandbox::new(
            egress,
            broker,
            audit.clone(),
        ));
        let proxy = Arc::new(sandbox::LoopbackCredentialProxy::new(secret_store.clone()));
        let approval = Arc::new(policy::ActionApprovalGate::new("aegis_default_approval_secret"));
        let delegation = Arc::new(policy::UserIdentityDelegationBroker::new());
        let oauth_connect = Arc::new(policy::VendorAgnosticOAuthRouter::new(delegation.clone()));
        let tier = std::env::var("AEGIS_POLICY_TIER")
            .ok()
            .and_then(|s| s.parse::<PolicyTier>().ok())
            .unwrap_or(PolicyTier::Hybrid);
        let max_egress = std::env::var("AEGIS_MAX_EGRESS_BYTES")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(10 * 1024 * 1024);

        let data_egress = Arc::new(policy::TieredDataEgressEngine::new(
            tier,
            max_egress,
            Some(approval.clone()),
        ));
        let in_situ = Arc::new(sandbox::SandboxedInSituEnclave::new(default_sandbox.clone()));

        let mut channels = vec![
            ApprovalChannelTarget::ConsoleLog,
            ApprovalChannelTarget::InBandMcp,
        ];
        if let Ok(url) = std::env::var("AEGIS_SLACK_WEBHOOK_URL") {
            channels.push(ApprovalChannelTarget::SlackWebhook {
                webhook_url: url,
                channel: std::env::var("AEGIS_SLACK_CHANNEL").ok(),
            });
        }
        if let Ok(url) = std::env::var("AEGIS_TEAMS_WEBHOOK_URL") {
            channels.push(ApprovalChannelTarget::TeamsWebhook { webhook_url: url });
        }
        if let Ok(url) = std::env::var("AEGIS_GENERIC_WEBHOOK_URL") {
            channels.push(ApprovalChannelTarget::GenericWebhook {
                url,
                secret_token: std::env::var("AEGIS_GENERIC_WEBHOOK_SECRET").ok(),
            });
        }
        let approval_dispatcher = Arc::new(policy::MultiChannelApprovalDispatcher::new(channels));
        let resume_router = Arc::new(policy::DurableTaskResumeRouter::new(approval.clone(), audit.clone()));

        Self {
            state,
            policy,
            dlp,
            audit,
            skills,
            sandbox: default_sandbox,
            drain: Arc::new(state::DrainCoordinator::new()),
            secret_store,
            proxy,
            approval,
            delegation,
            oauth_connect,
            data_egress,
            in_situ,
            approval_dispatcher,
            resume_router,
        }
    }

    pub fn with_sandbox(mut self, sandbox: Arc<dyn CodeSandboxEngine>) -> Self {
        self.sandbox = sandbox;
        self
    }

    pub fn sandbox(&self) -> &dyn CodeSandboxEngine {
        self.sandbox.as_ref()
    }

    pub fn with_drain(mut self, drain: Arc<state::DrainCoordinator>) -> Self {
        self.drain = drain;
        self
    }

    pub fn drain_coordinator(&self) -> &state::DrainCoordinator {
        &self.drain
    }

    pub fn drain_coordinator_arc(&self) -> &Arc<state::DrainCoordinator> {
        &self.drain
    }

    pub fn approval_gate(&self) -> &dyn ApprovalGate {
        self.approval.as_ref()
    }

    pub fn oauth_connect(&self) -> &dyn OAuthConnectEngine {
        self.oauth_connect.as_ref()
    }

    pub fn delegation_broker(&self) -> &dyn IdentityDelegationBroker {
        self.delegation.as_ref()
    }

    pub fn credential_proxy(&self) -> &dyn CredentialProxyEngine {
        self.proxy.as_ref()
    }

    pub fn secret_store(&self) -> &dyn SecretStore {
        self.secret_store.as_ref()
    }

    pub fn data_egress(&self) -> &dyn DataEgressPolicyEngine {
        self.data_egress.as_ref()
    }

    pub fn in_situ(&self) -> &dyn InSituDataEnclave {
        self.in_situ.as_ref()
    }

    pub fn approval_dispatcher(&self) -> &dyn ApprovalNotificationDispatcher {
        self.approval_dispatcher.as_ref()
    }

    pub fn resume_router(&self) -> &dyn DurableResumeRouter {
        self.resume_router.as_ref()
    }

    /// Discover tools with progressive disclosure projection
    pub async fn discover_tools(
        &self,
        tools: &[ToolDefinition],
        query: &str,
        tier: DisclosureTier,
    ) -> Vec<ProjectedTool> {
        let q_lower = query.to_lowercase();
        tools
            .iter()
            .filter(|t| {
                query == "*"
                    || query.is_empty()
                    || t.name.to_lowercase().contains(&q_lower)
                    || t.description.to_lowercase().contains(&q_lower)
                    || t.server.to_lowercase().contains(&q_lower)
                    || t.tags.iter().any(|tag| tag.to_lowercase().contains(&q_lower))
            })
            .map(|t| discovery::project_tool(t, tier, 1.0))
            .collect()
    }

    /// Enterprise Zero-Trust Tool Execution Pipeline (Asynchronous)
    pub async fn execute_tool_async<F, Fut>(
        &self,
        req: ToolCallRequest,
        raw_executor: F,
    ) -> AegisResult<ToolCallResponse>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = AegisResult<serde_json::Value>>,
    {
        run_tool_execution_pipeline(
            &self.state,
            &self.policy,
            &self.dlp,
            &self.audit,
            &self.drain,
            req,
            raw_executor,
        )
        .await
    }

    /// Synchronous convenience wrapper for execute_tool_async
    pub async fn execute_tool(
        &self,
        req: ToolCallRequest,
        raw_executor: impl FnOnce() -> AegisResult<serde_json::Value>,
    ) -> AegisResult<ToolCallResponse> {
        self.execute_tool_async(req, || async { raw_executor() }).await
    }

    /// Access the centralized skill registry
    pub fn skills(&self) -> &dyn SkillRegistry {
        self.skills.as_ref()
    }

    /// Access the policy engine
    pub fn policy(&self) -> &dyn PolicyEngine {
        self.policy.as_ref()
    }
}

impl Default for AegisGateway {
    fn default() -> Self {
        Self::new(
            Arc::new(state::InMemoryStateBackend::new()),
            Arc::new(policy::AbacPolicyEngine::new()),
            Arc::new(dlp::PiiDlpPipeline::new()),
            Arc::new(audit::StructuredAuditLogger::new()),
            Arc::new(skills::LocalSkillRegistry::new()),
        )
    }
}
