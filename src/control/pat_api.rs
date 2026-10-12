// SPDX-License-Identifier: MIT

use serde_json::json;

use crate::policy::pat_service::{get_shared_pat_service, CreatePatRequest};

/// Dispatcher for Enterprise Fine-Grained Personal Access Token (PAT) APIs
pub struct PatApiDispatcher;

impl PatApiDispatcher {
    pub async fn dispatch(
        method: &str,
        path: &str,
        body_bytes: &[u8],
    ) -> (u16, Vec<u8>, &'static str) {
        let pat_service = get_shared_pat_service();

        match (method, path) {
            // 1. List Active & Historical PATs for Current User
            ("GET", "/api/v1/auth/pat") | ("GET", "/api/v1/auth/pat/list") => {
                let subject = Self::extract_subject_from_headers(body_bytes);
                let pats = pat_service.list_pats(&subject);
                let body = serde_json::to_string(&pats).unwrap_or_default();
                (200, body.into_bytes(), "application/json")
            }

            // 2. Generate New Scoped PAT (One-Time Display)
            ("POST", "/api/v1/auth/pat") => {
                let req: CreatePatRequest = match serde_json::from_slice(body_bytes) {
                    Ok(r) => r,
                    Err(e) => {
                        let err = json!({
                            "error": "invalid_request",
                            "error_description": format!("Failed to parse JSON body: {}", e)
                        });
                        return (400, err.to_string().into_bytes(), "application/json");
                    }
                };

                let subject = Self::extract_subject_from_body(body_bytes)
                    .unwrap_or_else(|| Self::extract_subject_from_headers(body_bytes));

                match pat_service.create_pat(&subject, req) {
                    Ok(created) => {
                        let body = serde_json::to_string(&created).unwrap_or_default();
                        (201, body.into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({
                            "error": "creation_failed",
                            "error_description": e.to_string()
                        });
                        (400, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // 3. Revoke PAT by ID (Instant Remote Kill-Switch)
            ("POST", "/api/v1/auth/pat/revoke") => {
                let body_val: serde_json::Value = match serde_json::from_slice(body_bytes) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = json!({ "error": "invalid_request", "error_description": e.to_string() });
                        return (400, err.to_string().into_bytes(), "application/json");
                    }
                };

                let pat_id = body_val
                    .get("id")
                    .or_else(|| body_val.get("pat_id"))
                    .or_else(|| body_val.get("token_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                if pat_id.is_empty() {
                    let err = json!({ "error": "invalid_request", "error_description": "Field 'id' is required" });
                    return (400, err.to_string().into_bytes(), "application/json");
                }

                let subject = Self::extract_subject_from_headers(body_bytes);
                match pat_service.revoke_pat(&subject, pat_id) {
                    Ok(()) => {
                        let ok = json!({ "status": "revoked", "id": pat_id });
                        (200, ok.to_string().into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "revocation_failed", "error_description": e.to_string() });
                        (400, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // Dynamic Path Handling for DELETE /api/v1/auth/pat/{id}
            ("DELETE", p) if p.starts_with("/api/v1/auth/pat/") => {
                let pat_id = p.trim_start_matches("/api/v1/auth/pat/").trim();
                let subject = Self::extract_subject_from_headers(body_bytes);
                match pat_service.revoke_pat(&subject, pat_id) {
                    Ok(()) => {
                        let ok = json!({ "status": "revoked", "id": pat_id });
                        (200, ok.to_string().into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "revocation_failed", "error_description": e.to_string() });
                        (400, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            _ => {
                let err = json!({ "error": "not_found", "path": path });
                (404, err.to_string().into_bytes(), "application/json")
            }
        }
    }

    fn extract_subject_from_headers(body_bytes: &[u8]) -> String {
        let text = String::from_utf8_lossy(body_bytes);
        if let Some(pos) = text.find("propylon_jwt_") {
            let part = &text[pos + 13..];
            let sub = part.split('.').next().unwrap_or(part).split_whitespace().next().unwrap_or(part);
            if !sub.trim().is_empty() {
                return sub.trim().to_string();
            }
        }
        "developer@bps.go.id".to_string()
    }

    fn extract_subject_from_body(body_bytes: &[u8]) -> Option<String> {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body_bytes) {
            if let Some(s) = val.get("subject").and_then(|v| v.as_str()) {
                if !s.trim().is_empty() {
                    return Some(s.trim().to_string());
                }
            }
        }
        None
    }
}
