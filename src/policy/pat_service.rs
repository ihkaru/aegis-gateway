// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::core::error::{PropylonError, PropylonResult};
use crate::policy::jwt_service::get_shared_jwt_service;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalAccessToken {
    pub id: String,
    pub name: String,
    pub subject: String,
    pub scopes: Vec<String>,
    pub created_at: i64,
    pub expires_at: i64,
    pub last_used_at: Option<i64>,
    pub max_idle_secs: Option<i64>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePatRequest {
    pub name: String,
    pub ttl_days: Option<u32>,
    pub max_idle_days: Option<u32>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePatResponse {
    pub id: String,
    pub name: String,
    pub token: String,
    pub expires_at: i64,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatSummary {
    pub id: String,
    pub name: String,
    pub subject: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub last_used_at: Option<i64>,
    pub scopes: Vec<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct PatValidationResult {
    pub pat_id: String,
    pub subject: String,
    pub scopes: Vec<String>,
}

pub struct PatService {
    pats: Arc<RwLock<HashMap<String, PersonalAccessToken>>>,
}

static SHARED_PAT_SERVICE: OnceLock<Arc<PatService>> = OnceLock::new();

pub fn get_shared_pat_service() -> &'static Arc<PatService> {
    SHARED_PAT_SERVICE.get_or_init(|| Arc::new(PatService::new()))
}

impl PatService {
    pub fn new() -> Self {
        Self {
            pats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_pat(&self, subject: &str, req: CreatePatRequest) -> PropylonResult<CreatePatResponse> {
        let name = req.name.trim();
        if name.is_empty() {
            return Err(PropylonError::Validation("Token name cannot be empty".to_string()));
        }

        let ttl_days = req.ttl_days.unwrap_or(30).clamp(1, 365);
        let max_idle_days = req.max_idle_days.unwrap_or(14).clamp(1, 90);
        let now = chrono::Utc::now().timestamp();
        let ttl_secs = (ttl_days as i64) * 86400;
        let expires_at = now + ttl_secs;
        let max_idle_secs = Some((max_idle_days as i64) * 86400);

        let scopes = req.scopes.unwrap_or_else(|| {
            vec![
                "tools:execute:safe".to_string(),
                "execute_code".to_string(),
                "gateway_search_tools".to_string(),
            ]
        });

        let jwt_service = get_shared_jwt_service();
        let raw_jwt = jwt_service.generate_token(
            subject,
            vec!["pat_user".to_string(), "developer".to_string()],
            Some("Enterprise PAT".to_string()),
            Some("bps".to_string()),
            ttl_secs,
        );

        let claims = jwt_service.validate_token(&raw_jwt)?;
        let pat_id = claims.jti;
        let token = format!("prp_pat_{}", raw_jwt);

        let record = PersonalAccessToken {
            id: pat_id.clone(),
            name: name.to_string(),
            subject: subject.to_string(),
            scopes: scopes.clone(),
            created_at: now,
            expires_at,
            last_used_at: Some(now),
            max_idle_secs,
            revoked: false,
        };

        let mut lock = self.pats.write().map_err(|_| {
            PropylonError::Internal("Failed to acquire write lock on PAT registry".to_string())
        })?;
        lock.insert(pat_id.clone(), record);

        Ok(CreatePatResponse {
            id: pat_id,
            name: name.to_string(),
            token,
            expires_at,
            scopes,
        })
    }

    pub fn validate_pat(&self, token: &str, tool_name: Option<&str>) -> PropylonResult<PatValidationResult> {
        let raw_token = token.strip_prefix("prp_pat_").unwrap_or(token).trim();
        let jwt_service = get_shared_jwt_service();
        let claims = jwt_service.validate_token(raw_token)?;

        let now = chrono::Utc::now().timestamp();
        let mut lock = self.pats.write().map_err(|_| {
            PropylonError::Internal("Failed to acquire lock on PAT registry".to_string())
        })?;

        if let Some(record) = lock.get_mut(&claims.jti) {
            if record.revoked {
                return Err(PropylonError::AuthenticationFailed(
                    "Personal Access Token has been revoked".to_string(),
                ));
            }

            if let (Some(last_used), Some(max_idle)) = (record.last_used_at, record.max_idle_secs) {
                if now - last_used > max_idle {
                    record.revoked = true;
                    let _ = jwt_service.revoke_jti(&claims.jti);
                    return Err(PropylonError::AuthenticationFailed(
                        "Personal Access Token expired due to inactivity timeout (sliding window)".to_string(),
                    ));
                }
            }

            record.last_used_at = Some(now);

            if let Some(tool) = tool_name {
                Self::enforce_tool_scope(&record.scopes, tool)?;
            }

            Ok(PatValidationResult {
                pat_id: record.id.clone(),
                subject: record.subject.clone(),
                scopes: record.scopes.clone(),
            })
        } else {
            if let Some(tool) = tool_name {
                let default_scopes = vec!["tools:execute:safe".to_string()];
                Self::enforce_tool_scope(&default_scopes, tool)?;
            }

            Ok(PatValidationResult {
                pat_id: claims.jti,
                subject: claims.sub,
                scopes: vec!["tools:execute:safe".to_string()],
            })
        }
    }

    fn enforce_tool_scope(scopes: &[String], tool: &str) -> PropylonResult<()> {
        if scopes.iter().any(|s| s == "*" || s == "tools:all") {
            return Ok(());
        }

        if scopes.iter().any(|s| s == tool) {
            return Ok(());
        }

        if scopes.iter().any(|s| s == "tools:execute:safe") {
            let is_destructive = tool.contains("delete") || tool.contains("drop");
            if is_destructive {
                return Err(PropylonError::PolicyDenied(format!(
                    "PAT policy violation: Destructive operation '{}' requires elevated admin scopes",
                    tool
                )));
            }
            return Ok(());
        }

        Err(PropylonError::PolicyDenied(format!(
            "PAT policy violation: Tool '{}' is not permitted by assigned scopes {:?}",
            tool, scopes
        )))
    }

    pub fn list_pats(&self, subject: &str) -> Vec<PatSummary> {
        let now = chrono::Utc::now().timestamp();
        let lock = self.pats.read().map(|l| l.clone()).unwrap_or_default();
        let mut list: Vec<PatSummary> = lock
            .values()
            .filter(|p| p.subject == subject || subject == "admin" || subject.is_empty())
            .map(|p| {
                let is_active = !p.revoked && now < p.expires_at;
                PatSummary {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    subject: p.subject.clone(),
                    created_at: p.created_at,
                    expires_at: p.expires_at,
                    last_used_at: p.last_used_at,
                    scopes: p.scopes.clone(),
                    is_active,
                }
            })
            .collect();

        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    pub fn revoke_pat(&self, subject: &str, pat_id: &str) -> PropylonResult<()> {
        let mut lock = self.pats.write().map_err(|_| {
            PropylonError::Internal("Failed to acquire write lock on PAT registry".to_string())
        })?;

        if let Some(record) = lock.get_mut(pat_id) {
            if !subject.is_empty() && subject != "admin" && record.subject != subject {
                return Err(PropylonError::PolicyDenied(
                    "You do not have permission to revoke this PAT".to_string(),
                ));
            }
            record.revoked = true;
            let jwt_service = get_shared_jwt_service();
            let _ = jwt_service.revoke_jti(pat_id);
            return Ok(());
        }

        let jwt_service = get_shared_jwt_service();
        let _ = jwt_service.revoke_jti(pat_id);
        Ok(())
    }

    #[doc(hidden)]
    pub fn set_last_used_at_for_test(&self, pat_id: &str, ts: i64) {
        if let Ok(mut lock) = self.pats.write() {
            if let Some(record) = lock.get_mut(pat_id) {
                record.last_used_at = Some(ts);
            }
        }
    }
}

pub fn check_pat_tool_call_permission(auth_hdr: Option<&str>, body: &str) -> Option<String> {
    let raw_auth = auth_hdr?;
    let clean = raw_auth.strip_prefix("Bearer ").unwrap_or(raw_auth).trim();
    if !clean.starts_with("prp_pat_") {
        return None;
    }

    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    if v.get("method").and_then(|m| m.as_str()) != Some("tools/call") {
        return None;
    }

    let tool = v.get("params").and_then(|p| p.get("name")).and_then(|n| n.as_str());
    if let Err(e) = get_shared_pat_service().validate_pat(clean, tool) {
        let req_id = v.get("id").cloned().unwrap_or(serde_json::Value::Null);
        return Some(serde_json::json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {
                "code": -32003,
                "message": format!("Access Denied: {}", e)
            }
        }).to_string());
    }
    None
}

impl Default for PatService {
    fn default() -> Self {
        Self::new()
    }
}
