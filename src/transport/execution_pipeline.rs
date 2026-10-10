// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::time::Instant;

use crate::core::audit::{AuditAction, AuditEvent, AuditSink};
use crate::core::dlp::DlpPipeline;
use crate::core::error::{AegisError, AegisResult};
use crate::core::policy::{PolicyContext, PolicyDecision, PolicyEngine};
use crate::core::state::DistributedState;
use crate::core::types::{ToolCallRequest, ToolCallResponse};
use crate::state::DrainCoordinator;

/// Zero-Trust Tool Execution Pipeline
pub async fn run_tool_execution_pipeline<F, Fut>(
    state: &Arc<dyn DistributedState>,
    policy: &Arc<dyn PolicyEngine>,
    dlp: &Arc<dyn DlpPipeline>,
    audit: &Arc<dyn AuditSink>,
    drain: &Arc<DrainCoordinator>,
    req: ToolCallRequest,
    raw_executor: F,
) -> AegisResult<ToolCallResponse>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = AegisResult<serde_json::Value>>,
{
    let _task_slot = drain.acquire_slot()?;
    let start = Instant::now();

    // 1. Multi-Tenant Budget & Quota Check
    if !state.check_budget(&req.caller.tenant_id).await? {
        audit
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
    if !state.acquire(&req.caller.tenant_id, &req.tool, 1).await? {
        audit
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
    if !state.is_available(&req.server).await? {
        audit
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

    match policy.evaluate(&policy_ctx).await? {
        PolicyDecision::Deny { reason } => {
            audit
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
            state.record_success(&req.server).await?;
            out
        }
        Err(e) => {
            state.record_failure(&req.server).await?;
            return Err(e);
        }
    };

    // 6. Real-Time DLP & PII Sanitization
    let (sanitized_output, dlp_findings) = dlp.sanitize_response(raw_output).await?;
    let dlp_masked = !dlp_findings.is_empty();

    // 7. Tamper-Evident SIEM Audit Emission
    audit
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
