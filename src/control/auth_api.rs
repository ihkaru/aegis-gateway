// SPDX-License-Identifier: MIT

use serde_json::json;
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::control::abac_engine::LocalAbacEngine;
use crate::control::identity_broker::LocalIdentityBroker;
use crate::core::abac::{AbacContext, AbacPolicy, AbacPolicyEngine};
use crate::core::identity::{AssertionExchangeRequest, DeviceAuthRequest, IdentityFederationBroker};
use crate::core::types::{CallerContext, TenantId};

static BROKER_INSTANCE: OnceCell<Arc<LocalIdentityBroker>> = OnceCell::const_new();
static ABAC_INSTANCE: OnceCell<Arc<LocalAbacEngine>> = OnceCell::const_new();

async fn get_broker() -> &'static Arc<LocalIdentityBroker> {
    BROKER_INSTANCE
        .get_or_init(|| async {
            Arc::new(LocalIdentityBroker::new("https://aegis.corp.internal"))
        })
        .await
}

async fn get_abac() -> &'static Arc<LocalAbacEngine> {
    ABAC_INSTANCE
        .get_or_init(|| async {
            Arc::new(LocalAbacEngine::with_default_rules())
        })
        .await
}

/// Dispatcher for IAM, Device Authorization, and ABAC Policy APIs
pub struct AuthApiDispatcher;

impl AuthApiDispatcher {
    pub async fn dispatch(
        method: &str,
        path: &str,
        body_bytes: &[u8],
    ) -> (u16, Vec<u8>, &'static str) {
        match (method, path) {
            // RFC 8628: Initiate Device Authorization Grant
            ("POST", "/api/v1/auth/device/code") => {
                let req: DeviceAuthRequest = match serde_json::from_slice(body_bytes) {
                    Ok(r) => r,
                    Err(e) => {
                        let err = json!({ "error": "invalid_request", "error_description": e.to_string() });
                        return (400, err.to_string().into_bytes(), "application/json");
                    }
                };

                let broker = get_broker().await;
                match broker.start_device_flow(&req).await {
                    Ok(res) => {
                        let body = serde_json::to_string(&res).unwrap_or_default();
                        (200, body.into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "server_error", "error_description": e.to_string() });
                        (500, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // RFC 8628: Poll Device Flow Status / Token Issuance
            ("POST", "/api/v1/auth/device/token") => {
                let body_val: serde_json::Value = match serde_json::from_slice(body_bytes) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = json!({ "error": "invalid_request", "error_description": e.to_string() });
                        return (400, err.to_string().into_bytes(), "application/json");
                    }
                };

                let device_code = body_val.get("device_code").and_then(|v| v.as_str()).unwrap_or_default();
                let broker = get_broker().await;

                match broker.poll_device_flow(device_code).await {
                    Ok(crate::core::identity::DeviceCodeStatus::Pending) => {
                        let err = json!({ "error": "authorization_pending", "error_description": "User has not yet approved login" });
                        (400, err.to_string().into_bytes(), "application/json")
                    }
                    Ok(crate::core::identity::DeviceCodeStatus::Approved { claims }) => {
                        let token_res = json!({
                            "access_token": format!("aegis_jwt_{}", claims.subject),
                            "token_type": "Bearer",
                            "expires_in": 3600,
                            "scope": "mcp:all",
                            "claims": claims
                        });
                        (200, token_res.to_string().into_bytes(), "application/json")
                    }
                    Ok(crate::core::identity::DeviceCodeStatus::Denied { reason }) => {
                        let err = json!({ "error": "access_denied", "error_description": reason });
                        (403, err.to_string().into_bytes(), "application/json")
                    }
                    Ok(crate::core::identity::DeviceCodeStatus::Expired) => {
                        let err = json!({ "error": "expired_token", "error_description": "Device code has expired" });
                        (400, err.to_string().into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "invalid_grant", "error_description": e.to_string() });
                        (404, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // Internal / IdP Callback: Approve Device Flow
            ("POST", "/api/v1/auth/device/approve") => {
                let body_val: serde_json::Value = match serde_json::from_slice(body_bytes) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = json!({ "error": "invalid_request", "error_description": e.to_string() });
                        return (400, err.to_string().into_bytes(), "application/json");
                    }
                };

                let user_code = body_val.get("user_code").and_then(|v| v.as_str()).unwrap_or_default();
                let subject = body_val.get("subject").and_then(|v| v.as_str()).unwrap_or("enterprise_user");
                let department = body_val.get("department").and_then(|v| v.as_str()).unwrap_or("Engineering");

                let context = CallerContext {
                    tenant_id: TenantId::new("default"),
                    subject: subject.to_string(),
                    roles: vec!["developer".to_string()],
                    department: Some(department.to_string()),
                    client_ip: Some("127.0.0.1".to_string()),
                    session_id: format!("sess_{}", user_code),
                };

                let broker = get_broker().await;
                match broker.approve_device_flow(user_code, context).await {
                    Ok(()) => {
                        let ok = json!({ "status": "approved", "user_code": user_code });
                        (200, ok.to_string().into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "approval_failed", "error_description": e.to_string() });
                        (404, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // RFC 8693: Token Exchange / SAML & OIDC Assertion Ingestion
            ("POST", "/api/v1/auth/token/exchange") => {
                let req: AssertionExchangeRequest = match serde_json::from_slice(body_bytes) {
                    Ok(r) => r,
                    Err(e) => {
                        let err = json!({ "error": "invalid_request", "error_description": e.to_string() });
                        return (400, err.to_string().into_bytes(), "application/json");
                    }
                };

                let broker = get_broker().await;
                match broker.exchange_assertion(&req).await {
                    Ok(token) => {
                        let res = serde_json::to_string(&token).unwrap_or_default();
                        (200, res.into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "invalid_grant", "error_description": e.to_string() });
                        (400, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // Dry-Run ABAC Policy Simulation
            ("POST", "/api/v1/policy/simulate") => {
                let ctx: AbacContext = match serde_json::from_slice(body_bytes) {
                    Ok(c) => c,
                    Err(e) => {
                        let err = json!({ "error": "invalid_context", "error_description": e.to_string() });
                        return (400, err.to_string().into_bytes(), "application/json");
                    }
                };

                let abac = get_abac().await;
                match abac.simulate(&ctx).await {
                    Ok(sim) => {
                        let res = serde_json::to_string(&sim).unwrap_or_default();
                        (200, res.into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "simulation_failed", "error_description": e.to_string() });
                        (500, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // Get Current Active ABAC Policy
            ("GET", "/api/v1/policy/abac") => {
                let abac = get_abac().await;
                match abac.get_active_policy().await {
                    Ok(policy) => {
                        let res = serde_json::to_string(&policy).unwrap_or_default();
                        (200, res.into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "fetch_failed", "error_description": e.to_string() });
                        (500, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // Hot-Reload Active ABAC Policy
            ("POST", "/api/v1/policy/abac") => {
                let policy: AbacPolicy = match serde_json::from_slice(body_bytes) {
                    Ok(p) => p,
                    Err(e) => {
                        let err = json!({ "error": "invalid_policy", "error_description": e.to_string() });
                        return (400, err.to_string().into_bytes(), "application/json");
                    }
                };

                let abac = get_abac().await;
                match abac.reload_policy(policy).await {
                    Ok(()) => {
                        let ok = json!({ "status": "reloaded" });
                        (200, ok.to_string().into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "reload_failed", "error_description": e.to_string() });
                        (500, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            _ => {
                let err = json!({ "error": "not_found", "path": path });
                (404, err.to_string().into_bytes(), "application/json")
            }
        }
    }
}
