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
pub mod policy;
pub mod sandbox;
pub mod skills;
pub mod state;
pub mod transport;

use std::sync::Arc;
use std::time::Instant;

use crate::core::audit::{AuditAction, AuditEvent, AuditSink};
use crate::core::dlp::DlpPipeline;
use crate::core::error::{AegisError, AegisResult};
use crate::core::policy::{PolicyContext, PolicyDecision, PolicyEngine};
use crate::core::sandbox::CodeSandboxEngine;
use crate::core::skills::SkillRegistry;
use crate::core::state::DistributedState;
use crate::core::types::{
    DisclosureTier, ProjectedTool, ToolCallRequest, ToolCallResponse, ToolDefinition,
};

/// High-level Enterprise Gateway Orchestrator (Dependency Inversion applied)
pub struct AegisGateway {
    state: Arc<dyn DistributedState>,
    policy: Arc<dyn PolicyEngine>,
    dlp: Arc<dyn DlpPipeline>,
    audit: Arc<dyn AuditSink>,
    skills: Arc<dyn SkillRegistry>,
    sandbox: Arc<dyn CodeSandboxEngine>,
    drain: Arc<state::DrainCoordinator>,
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
        let secret_store = Arc::new(policy::secrets::EnvSecretStore::new());
        let broker = Arc::new(sandbox::VaultCredentialBroker::new(secret_store));
        let default_sandbox = Arc::new(sandbox::HermeticProcessSandbox::new(
            egress,
            broker,
            audit.clone(),
        ));
        Self {
            state,
            policy,
            dlp,
            audit,
            skills,
            sandbox: default_sandbox,
            drain: Arc::new(state::DrainCoordinator::new()),
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
        let _task_slot = self.drain.acquire_slot()?;
        let start = Instant::now();

        // 1. Multi-Tenant Budget & Quota Check
        if !self.state.check_budget(&req.caller.tenant_id).await? {
            self.audit
                .emit(&AuditEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    caller: req.caller.clone(),
                    action: AuditAction::PolicyEvaluated {
                        allowed: false,
                        reason: Some("Monthly spending quota exceeded".to_string()),
                    },
                    target_resource: format!("{}:{}", req.server, req.tool),
                    payload_hash_sha256: "refusal_quota".to_string(),
                    metadata: serde_json::json!({ "refusal": "QuotaExceeded", "args": req.arguments }),
                })
                .await?;
            return Err(AegisError::RateLimitExceeded {
                tenant: req.caller.tenant_id.as_str().to_string(),
                message: "Monthly spending quota exceeded".to_string(),
            });
        }

        // 2. Distributed Rate Limiter
        if !self
            .state
            .acquire(&req.caller.tenant_id, &req.tool, 1)
            .await?
        {
            self.audit
                .emit(&AuditEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    caller: req.caller.clone(),
                    action: AuditAction::PolicyEvaluated {
                        allowed: false,
                        reason: Some("Too many concurrent requests".to_string()),
                    },
                    target_resource: format!("{}:{}", req.server, req.tool),
                    payload_hash_sha256: "refusal_ratelimit".to_string(),
                    metadata: serde_json::json!({ "refusal": "RateLimitExceeded", "args": req.arguments }),
                })
                .await?;
            return Err(AegisError::RateLimitExceeded {
                tenant: req.caller.tenant_id.as_str().to_string(),
                message: "Too many concurrent requests".to_string(),
            });
        }

        // 3. Distributed Circuit Breaker Check
        if !self.state.is_available(&req.server).await? {
            self.audit
                .emit(&AuditEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    caller: req.caller.clone(),
                    action: AuditAction::PolicyEvaluated {
                        allowed: false,
                        reason: Some(format!("Circuit open for {}", req.server)),
                    },
                    target_resource: format!("{}:{}", req.server, req.tool),
                    payload_hash_sha256: "refusal_circuit".to_string(),
                    metadata: serde_json::json!({ "refusal": "CircuitOpen", "server": req.server }),
                })
                .await?;
            return Err(AegisError::CircuitOpen(req.server.clone()));
        }

        // 4. Granular ABAC Policy Evaluation
        let policy_ctx = PolicyContext {
            caller: req.caller.clone(),
            server: req.server.clone(),
            tool: req.tool.clone(),
            arguments: req.arguments.clone(),
            requested_at: chrono::Utc::now(),
        };

        match self.policy.evaluate(&policy_ctx).await? {
            PolicyDecision::Deny { reason } => {
                self.audit
                    .emit(&AuditEvent {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        timestamp: chrono::Utc::now(),
                        caller: req.caller.clone(),
                        action: AuditAction::PolicyEvaluated {
                            allowed: false,
                            reason: Some(reason.clone()),
                        },
                        target_resource: format!("{}:{}", req.server, req.tool),
                        payload_hash_sha256: "policy_denied".to_string(),
                        metadata: serde_json::json!({ "arguments": req.arguments }),
                    })
                    .await?;
                return Err(AegisError::PolicyDenied(reason));
            }
            PolicyDecision::Allow => {}
        }

        // 5. Execute Tool Backend Asynchronously
        let raw_output = match raw_executor().await {
            Ok(out) => {
                self.state.record_success(&req.server).await?;
                out
            }
            Err(e) => {
                self.state.record_failure(&req.server).await?;
                return Err(e);
            }
        };

        // 6. Real-Time DLP & PII Sanitization
        let (sanitized_output, dlp_findings) = self.dlp.sanitize_response(raw_output).await?;
        let dlp_masked = !dlp_findings.is_empty();

        // 7. Tamper-Evident SIEM Audit Emission
        self.audit
            .emit(&AuditEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                caller: req.caller.clone(),
                action: AuditAction::ToolInvoked,
                target_resource: format!("{}:{}", req.server, req.tool),
                payload_hash_sha256: "verified".to_string(),
                metadata: serde_json::json!({
                    "dlp_masked": dlp_masked,
                    "findings_count": dlp_findings.len(),
                    "latency_ms": start.elapsed().as_millis() as u64
                }),
            })
            .await?;

        Ok(ToolCallResponse {
            success: true,
            output: sanitized_output,
            latency_ms: start.elapsed().as_millis() as u64,
            dlp_masked,
            attestation: None,
        })
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
