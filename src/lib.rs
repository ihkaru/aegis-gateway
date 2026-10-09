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
pub mod skills;
pub mod state;
pub mod transport;


use std::sync::Arc;
use std::time::Instant;

use crate::core::audit::{AuditAction, AuditEvent, AuditSink};
use crate::core::dlp::DlpPipeline;
use crate::core::error::{AegisError, AegisResult};
use crate::core::policy::{PolicyContext, PolicyDecision, PolicyEngine};
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
        Self {
            state,
            policy,
            dlp,
            audit,
            skills,
            drain: Arc::new(state::DrainCoordinator::new()),
        }
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
                    || t.name.to_lowercase().contains(&q_lower)
                    || t.description.to_lowercase().contains(&q_lower)
            })
            .map(|t| discovery::project_tool(t, tier, 1.0))
            .collect()
    }

    /// Enterprise Zero-Trust Tool Execution Pipeline
    pub async fn execute_tool(
        &self,
        req: ToolCallRequest,
        raw_executor: impl FnOnce() -> AegisResult<serde_json::Value>,
    ) -> AegisResult<ToolCallResponse> {
        let _task_slot = self.drain.acquire_slot()?;
        let start = Instant::now();


        // 1. Multi-Tenant Budget & Quota Check
        if !self.state.check_budget(&req.caller.tenant_id).await? {
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
            return Err(AegisError::RateLimitExceeded {
                tenant: req.caller.tenant_id.as_str().to_string(),
                message: "Too many concurrent requests".to_string(),
            });
        }

        // 3. Distributed Circuit Breaker Check
        if !self.state.is_available(&req.server).await? {
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

        // 5. Execute Tool Backend
        let raw_output = match raw_executor() {
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
        })
    }

    /// Access the centralized skill registry
    pub fn skills(&self) -> &dyn SkillRegistry {
        self.skills.as_ref()
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
