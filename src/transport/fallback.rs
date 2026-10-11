// SPDX-License-Identifier: MIT

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::core::profile::EnterpriseRuntimeProfile;

/// Fallback handler for UI static assets and control plane REST APIs.
pub async fn handle_fallback_ui_and_control_api(
    uri: axum::http::Uri,
    method: axum::http::Method,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let path = uri.path();
    let method_str = method.as_str();
    let profile = EnterpriseRuntimeProfile::from_env();

    // 1. Control plane REST API & SAML endpoints
    if path.starts_with("/api/v1/") || path.starts_with("/saml/") {
        let mut dispatch_body = body.to_vec();
        if path == "/api/v1/auth/session" && dispatch_body.is_empty() {
            if let Some(auth_hdr) = headers.get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
                dispatch_body = auth_hdr.as_bytes().to_vec();
            }
        }

        let (status_code, body_bytes, content_type) =
            crate::control::web_api::WebApiDispatcher::dispatch(
                method_str,
                path,
                &dispatch_body,
                std::time::Instant::now(),
                None,
                None,
                crate::transport::server::get_shared_invocations(),
                crate::transport::server::get_shared_dlp_events(),
                crate::transport::server::get_shared_policy_tier(),
            ).await;

        if status_code == 302 || status_code == 303 {
            let mut resp = StatusCode::FOUND.into_response();
            if let Ok(loc) = axum::http::HeaderValue::from_str(&String::from_utf8_lossy(&body_bytes)) {
                resp.headers_mut().insert(axum::http::header::LOCATION, loc);
            }
            return resp;
        }

        let mut resp = (
            StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK),
            body_bytes,
        ).into_response();
        resp.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static(content_type),
        );
        return resp;
    }

    // 2. Embedded Admin Web UI (With Opt-Out Check)
    if !profile.ui_enabled {
        let payload = json!({
            "error": "not_found",
            "message": "Embedded Web UI is disabled in headless mode. Set PROPYLON_UI_ENABLED=true or remove --no-ui flag to enable."
        });
        let mut resp = (StatusCode::NOT_FOUND, payload.to_string().into_bytes()).into_response();
        resp.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        return resp;
    }

    let admin_server = crate::control::web_ui::EmbeddedAdminServer::new(8080, true);
    let (status_code, body_bytes, content_type) = admin_server.handle_request(method_str, path).await;

    let mut resp = (
        StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK),
        body_bytes,
    ).into_response();
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static(content_type),
    );
    resp
}
