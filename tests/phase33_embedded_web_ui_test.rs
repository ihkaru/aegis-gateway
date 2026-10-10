use aegis_gateway::control::web_ui::EmbeddedAdminServer;

#[test]
fn test_embedded_admin_server_serves_spa_assets() {
    let server = EmbeddedAdminServer::new(8485, true);

    // Root path should serve index.html
    let (status, body, mime) = server.handle_request("GET", "/");
    assert_eq!(status, 200);
    assert_eq!(mime, "text/html; charset=utf-8");
    let html = String::from_utf8(body).expect("Invalid UTF-8");
    assert!(html.contains("Aegis Gateway | Control Plane Dashboard"));
    assert!(html.contains("id=\"app\""));

    // Explicit index.html path
    let (status_index, body_index, _) = server.handle_request("GET", "/index.html");
    assert_eq!(status_index, 200);
    let html_index = String::from_utf8(body_index).unwrap();
    assert!(html_index.contains("data-theme=\"zinc\""));
}

#[test]
fn test_embedded_admin_server_spa_client_routing_fallback() {
    let server = EmbeddedAdminServer::new(8485, true);

    // Deep client-side routes should fallback to index.html without 404
    for route in &["/dashboard", "/dlp", "/hitl", "/finops", "/topology/cluster"] {
        let (status, body, mime) = server.handle_request("GET", route);
        assert_eq!(status, 200, "Route {} failed to serve SPA fallback", route);
        assert_eq!(mime, "text/html; charset=utf-8");
        let html = String::from_utf8(body).unwrap();
        assert!(html.contains("id=\"app\""));
    }
}

#[test]
fn test_embedded_admin_server_rest_api_endpoints() {
    let server = EmbeddedAdminServer::new(8485, true);

    // 1. Overview API
    let (status, body, mime) = server.handle_request("GET", "/api/v1/overview");
    assert_eq!(status, 200);
    assert_eq!(mime, "application/json");
    let json_str = String::from_utf8(body).unwrap();
    assert!(json_str.contains("HEALTHY"));
    assert!(json_str.contains("1420"));

    // 2. DLP Events API
    let (status_dlp, body_dlp, _) = server.handle_request("GET", "/api/v1/dlp/events");
    assert_eq!(status_dlp, 200);
    let dlp_str = String::from_utf8(body_dlp).unwrap();
    assert!(dlp_str.contains("REDACTED_LUHN_MASK"));

    // 3. HITL Tasks Queue API
    let (status_hitl, body_hitl, _) = server.handle_request("GET", "/api/v1/hitl/queue");
    assert_eq!(status_hitl, 200);
    let hitl_str = String::from_utf8(body_hitl).unwrap();
    assert!(hitl_str.contains("hitl_tkt_8091"));

    // 4. FinOps API
    let (status_finops, body_finops, _) = server.handle_request("GET", "/api/v1/finops");
    assert_eq!(status_finops, 200);
    let finops_str = String::from_utf8(body_finops).unwrap();
    assert!(finops_str.contains("frozen_tenants"));
}

#[test]
fn test_admin_server_disabled_by_policy() {
    let disabled_server = EmbeddedAdminServer::new(8485, false);

    let (status, body, _) = disabled_server.handle_request("GET", "/");
    assert_eq!(status, 404);
    let msg = String::from_utf8(body).unwrap();
    assert!(msg.contains("disabled by policy"));
}
