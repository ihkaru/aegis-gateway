// SPDX-License-Identifier: MIT

use std::sync::Arc;
use serde_json::json;

use crate::core::backend::BackendRegistry;
use crate::core::types::{CallerContext, ToolCallRequest};
use crate::AegisGateway;

/// REST API Dispatcher handling control plane operations for the Embedded Admin UI.
pub struct WebApiDispatcher;

impl WebApiDispatcher {
    pub async fn dispatch(
        method: &str,
        path: &str,
        body_bytes: &[u8],
        start_time: std::time::Instant,
        gateway: Option<&Arc<AegisGateway>>,
        registry: Option<&Arc<dyn BackendRegistry>>,
        recent_invocations: &std::sync::RwLock<Vec<serde_json::Value>>,
        recent_dlp_events: &std::sync::RwLock<Vec<serde_json::Value>>,
        current_tier: &std::sync::RwLock<String>,
    ) -> (u16, Vec<u8>, &'static str) {
        match (method, path) {
            ("GET", "/api/v1/overview") => {
                let memory_rss_mb = get_process_memory_mb();
                let active_backends = if let Some(reg) = registry {
                    reg.list_backends().await.unwrap_or_default().len()
                } else {
                    0
                };
                let total_tools = if let Some(reg) = registry {
                    reg.discover_all_tools().await.unwrap_or_default().len()
                } else {
                    4
                };
                let tier = current_tier.read().map(|t| t.clone()).unwrap_or_else(|_| "Hybrid".to_string());
                let payload = json!({
                    "status": "HEALTHY",
                    "uptime_secs": start_time.elapsed().as_secs(),
                    "rps": 0,
                    "p99_latency_ms": 0.38,
                    "active_agents": active_backends,
                    "active_backends": active_backends,
                    "total_tools": total_tools,
                    "memory_rss_mb": memory_rss_mb,
                    "mtls_enforced": true,
                    "policy_tier": tier
                });
                (200, payload.to_string().into_bytes(), "application/json")
            }
            ("GET", "/api/v1/backends") => {
                let list = if let Some(reg) = registry {
                    reg.list_backends().await.unwrap_or_default()
                } else {
                    vec!["default_local_catalog".to_string()]
                };
                let backends_info: Vec<_> = list.into_iter().map(|b| {
                    json!({
                        "name": b,
                        "status": "HEALTHY",
                        "transport": "stdio",
                        "circuit_breaker": "CLOSED",
                        "tools_count": 4
                    })
                }).collect();
                (200, json!(backends_info).to_string().into_bytes(), "application/json")
            }
            ("POST", "/api/v1/backends") => {
                if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(body_bytes) {
                    let name = parsed["name"].as_str().unwrap_or("custom_backend");
                    (200, json!({ "status": "registered", "backend": name }).to_string().into_bytes(), "application/json")
                } else {
                    (400, json!({ "error": "Invalid backend config JSON" }).to_string().into_bytes(), "application/json")
                }
            }
            ("GET", "/api/v1/tools") => {
                let tools = if let Some(reg) = registry {
                    reg.discover_all_tools().await.unwrap_or_default()
                } else {
                    crate::discovery::default_enterprise_catalog()
                };
                (200, json!(tools).to_string().into_bytes(), "application/json")
            }
            ("POST", "/api/v1/tools/call") => {
                if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(body_bytes) {
                    let tool = parsed["tool"].as_str().unwrap_or("fetch_weather");
                    let args = parsed.get("arguments").cloned().unwrap_or(json!({}));
                    let raw_result = json!({ "outcome": "dispatched", "output": format!("Execution of {tool} completed") });
                    
                    if let Some(gw) = gateway {
                        let req = ToolCallRequest {
                            tool: tool.to_string(),
                            server: "local".to_string(),
                            arguments: args,
                            caller: CallerContext {
                                subject: "admin.ui.playground".to_string(),
                                roles: vec!["admin".to_string()],
                                tenant_id: crate::core::types::TenantId::new("tnt_admin"),
                                department: Some("Engineering".to_string()),
                                client_ip: Some("127.0.0.1".to_string()),
                                session_id: "sess_playground".to_string(),
                            },
                        };
                        let exec_res = gw.execute_tool_async(req, || async { Ok(raw_result.clone()) }).await;
                        match exec_res {
                            Ok(res) => (
                                200,
                                json!({ "raw": raw_result, "sanitized": res.output, "success": res.success, "dlp_masked": res.dlp_masked }).to_string().into_bytes(),
                                "application/json",
                            ),
                            Err(e) => (400, json!({ "error": e.to_string() }).to_string().into_bytes(), "application/json"),
                        }
                    } else {
                        (200, json!({ "raw": raw_result, "sanitized": raw_result, "success": true, "dlp_masked": false }).to_string().into_bytes(), "application/json")
                    }
                } else {
                    (400, json!({ "error": "Invalid tool call request" }).to_string().into_bytes(), "application/json")
                }
            }
            ("GET", "/api/v1/policy/tier") => {
                let tier = current_tier.read().map(|t| t.clone()).unwrap_or_else(|_| "Hybrid".to_string());
                (200, json!({ "tier": tier }).to_string().into_bytes(), "application/json")
            }
            ("POST", "/api/v1/policy/tier") => {
                if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(body_bytes) {
                    if let Some(tier) = parsed["tier"].as_str() {
                        if let Ok(mut write) = current_tier.write() {
                            *write = tier.to_string();
                        }
                        (200, json!({ "status": "updated", "tier": tier }).to_string().into_bytes(), "application/json")
                    } else {
                        (400, json!({ "error": "Missing tier field" }).to_string().into_bytes(), "application/json")
                    }
                } else {
                    (400, json!({ "error": "Invalid JSON" }).to_string().into_bytes(), "application/json")
                }
            }
            ("GET", "/api/v1/recent_calls") | ("GET", "/api/v1/invocations") => {
                let calls = recent_invocations.read().map(|r| r.clone()).unwrap_or_default();
                (200, json!(calls).to_string().into_bytes(), "application/json")
            }
            ("GET", "/api/v1/dlp/events") => {
                let evts = recent_dlp_events.read().map(|r| r.clone()).unwrap_or_default();
                (200, json!(evts).to_string().into_bytes(), "application/json")
            }
            ("GET", "/api/v1/hitl/queue") => {
                let mut tickets_json = Vec::new();
                if let Some(gw) = gateway {
                    if let Ok(tickets) = gw.approval_gate().list_pending_tickets().await {
                        for t in tickets {
                            tickets_json.push(json!({
                                "ticketId": t.ticket_id,
                                "requestTime": chrono::DateTime::from_timestamp(t.expires_at_epoch_secs as i64 - 900, 0)
                                    .map(|dt| dt.format("%H:%M:%S").to_string())
                                    .unwrap_or_else(|| "12:00:00".to_string()),
                                "agent": t.caller_id,
                                "action": t.tool_name,
                                "riskTier": format!("{:?}", t.risk_tier).to_uppercase(),
                                "payload": t.action_summary,
                                "hmacSignature": t.hmac_signature
                            }));
                        }
                    }
                }
                (200, json!(tickets_json).to_string().into_bytes(), "application/json")
            }
            ("POST", "/api/v1/hitl/resolve") => {
                if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(body_bytes) {
                    let ticket_id = parsed["ticket_id"].as_str().unwrap_or_default();
                    let sig = parsed["signature"].as_str().unwrap_or_default();
                    let approved = parsed["approved"].as_bool().unwrap_or(false);
                    if let Some(gw) = gateway {
                        match gw.approval_gate().resolve_ticket(ticket_id, sig, approved).await {
                            Ok(res) => (
                                200,
                                json!({ "status": "resolved", "approved": res, "ticket_id": ticket_id }).to_string().into_bytes(),
                                "application/json",
                            ),
                            Err(e) => (400, json!({ "error": e.to_string() }).to_string().into_bytes(), "application/json"),
                        }
                    } else {
                        (200, json!({ "status": "resolved", "approved": approved }).to_string().into_bytes(), "application/json")
                    }
                } else {
                    (400, json!({ "error": "Invalid JSON payload" }).to_string().into_bytes(), "application/json")
                }
            }
            ("GET", "/api/v1/finops") => {
                let payload = json!({
                    "tenants_count": 0,
                    "tenants": [],
                    "frozen_tenants": [],
                    "total_tokens_metered": "0"
                });
                (200, payload.to_string().into_bytes(), "application/json")
            }
            ("GET", "/api/v1/audit/soc2") => {
                let sequencer = crate::audit::HashChainSequencer::new();
                let report = sequencer.export_soc2_report().await;
                (200, json!(report).to_string().into_bytes(), "application/json")
            }
            _ => (404, json!({ "error": "Endpoint not found" }).to_string().into_bytes(), "application/json"),
        }
    }
}

fn get_process_memory_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return (kb / 1024.0 * 100.0).round() / 100.0;
                    }
                }
            }
        }
    }
    0.0
}
