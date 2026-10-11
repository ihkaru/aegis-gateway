// SPDX-License-Identifier: MIT

#![deny(unsafe_code)]

use std::fs;
use std::path::Path;
use std::sync::Arc;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use propylon::core::transport::WireProtocolHandler;
use propylon::state::DrainCoordinator;
use propylon::transport::{LiveHttpServer, McpProtocolHandler};
use propylon::PropylonGateway;

#[test]
fn test_docker_compose_enterprise_conformance() {
    let compose_path = Path::new("deploy/docker-compose.yml");
    assert!(compose_path.exists(), "docker-compose.yml must exist");

    let content = fs::read_to_string(compose_path).expect("read docker-compose.yml");
    let val: serde_yaml::Value = serde_yaml::from_str(&content).expect("valid yaml");

    let services = val.get("services").expect("services section");
    let propylon = services.get("propylon").expect("propylon service");

    // Non-root / rootless enforcement
    let user = propylon.get("user").and_then(|u| u.as_str());
    assert_eq!(user, Some("10001:10001"), "must enforce rootless UID:GID 10001");

    // Port multiplexing
    let ports = propylon.get("ports").and_then(|p| p.as_sequence()).expect("ports list");
    assert!(
        ports.iter().any(|p| p.as_str() == Some("8080:8080")),
        "must expose unified port 8080"
    );

    // Healthcheck probe
    let healthcheck = propylon.get("healthcheck").expect("healthcheck definition");
    let test_cmd = format!("{:?}", healthcheck.get("test"));
    assert!(
        test_cmd.contains("/healthz"),
        "healthcheck must probe /healthz endpoint"
    );

    // 12-factor environment variables
    let env = propylon.get("environment").expect("environment section");
    assert_eq!(env.get("PORT").and_then(|v| v.as_str()), Some("8080"));
    assert_eq!(env.get("PROPYLON_PORT").and_then(|v| v.as_str()), Some("8080"));
    assert_eq!(env.get("SAML_IDP_PROVIDER_NAME").and_then(|v| v.as_str()), Some("rIDM"));
}

#[test]
fn test_helm_chart_and_values_conformance() {
    let chart_path = Path::new("deploy/helm/propylon/Chart.yaml");
    assert!(chart_path.exists(), "Chart.yaml must exist");
    let chart_content = fs::read_to_string(chart_path).expect("read Chart.yaml");
    let chart_val: serde_yaml::Value = serde_yaml::from_str(&chart_content).expect("valid yaml");

    assert_eq!(chart_val.get("name").and_then(|v| v.as_str()), Some("propylon"));
    assert_eq!(chart_val.get("apiVersion").and_then(|v| v.as_str()), Some("v2"));
    assert_eq!(chart_val.get("type").and_then(|v| v.as_str()), Some("application"));

    let values_path = Path::new("deploy/helm/propylon/values.yaml");
    assert!(values_path.exists(), "values.yaml must exist");
    let values_content = fs::read_to_string(values_path).expect("read values.yaml");
    let values_val: serde_yaml::Value = serde_yaml::from_str(&values_content).expect("valid yaml");

    // Security context checks
    let pod_sec = values_val.get("podSecurityContext").expect("podSecurityContext");
    assert_eq!(pod_sec.get("runAsNonRoot").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(pod_sec.get("runAsUser").and_then(|v| v.as_i64()), Some(10001));

    let container_sec = values_val.get("securityContext").expect("securityContext");
    let caps_drop = container_sec
        .get("capabilities")
        .and_then(|c| c.get("drop"))
        .and_then(|d| d.as_sequence())
        .expect("drop capabilities");
    assert!(caps_drop.iter().any(|c| c.as_str() == Some("ALL")));

    // Unified port configuration
    let svc = values_val.get("service").expect("service config");
    assert_eq!(svc.get("port").and_then(|p| p.as_i64()), Some(8080));
    assert_eq!(svc.get("targetPort").and_then(|p| p.as_i64()), Some(8080));

    // Template files verification
    let templates = [
        "deploy/helm/propylon/templates/_helpers.tpl",
        "deploy/helm/propylon/templates/deployment.yaml",
        "deploy/helm/propylon/templates/service.yaml",
        "deploy/helm/propylon/templates/ingress.yaml",
        "deploy/helm/propylon/templates/serviceaccount.yaml",
    ];
    for tmpl in templates {
        assert!(Path::new(tmpl).exists(), "template {} must exist", tmpl);
    }
}

#[tokio::test]
async fn test_unified_single_port_multiplexing_router() {
    let gateway = Arc::new(PropylonGateway::default());
    let handler: Arc<dyn WireProtocolHandler> = Arc::new(McpProtocolHandler::new(gateway));
    let drain = Arc::new(DrainCoordinator::new());
    let server = LiveHttpServer::new(handler, drain, "127.0.0.1", 39550).expect("server init");
    let app = server.router();

    // 1. Healthz probe endpoint
    let req = Request::builder()
        .method("GET")
        .uri("/healthz")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "healthy");

    // 2. Readyz probe endpoint
    let req = Request::builder()
        .method("GET")
        .uri("/readyz")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "ready");

    // 3. Stats endpoint
    let req = Request::builder()
        .method("GET")
        .uri("/stats")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. SAML Metadata endpoint on same port
    let req = Request::builder()
        .method("GET")
        .uri("/saml/metadata")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap().to_str().unwrap(),
        "application/samlmetadata+xml"
    );

    // 5. SAML Config REST API on same port
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/saml/config")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["provider"], "rIDM");

    // 6. UI root HTML fallback on same port
    let req = Request::builder()
        .method("GET")
        .uri("/")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("<!DOCTYPE html>") || html.contains("<html"));
}
