// SPDX-License-Identifier: MIT

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use propylon::control::auth_api::AuthApiDispatcher;
use propylon::control::saml_api::SamlApiDispatcher;

#[tokio::test]
async fn test_ridm_saml_config_endpoint() {
    let (status, body, content_type) = SamlApiDispatcher::dispatch(
        "GET",
        "/api/v1/auth/saml/config",
        b"",
    ).await;

    assert_eq!(status, 200);
    assert_eq!(content_type, "application/json");

    let val: serde_json::Value = serde_json::from_slice(&body).expect("valid json");
    assert_eq!(val["provider"], "rIDM");
    assert_eq!(val["tenant"], "bps");
    assert_eq!(val["idp_entity_id"], "https://auth.dvlpid.my.id/t/bps");
    assert_eq!(val["idp_sso_url"], "https://auth.dvlpid.my.id/t/bps/saml/sso");
    assert_eq!(val["idp_metadata_url"], "https://auth.dvlpid.my.id/t/bps/saml/metadata");
    assert_eq!(val["sp_entity_id"], "https://mcp.dvlpid.my.id/saml/metadata");
    assert_eq!(val["sp_acs_url"], "https://mcp.dvlpid.my.id/api/v1/auth/saml/acs");
}

#[tokio::test]
async fn test_ridm_saml_metadata_endpoint() {
    let (status, body, content_type) = SamlApiDispatcher::dispatch(
        "GET",
        "/api/v1/auth/saml/metadata",
        b"",
    ).await;

    assert_eq!(status, 200);
    assert_eq!(content_type, "application/samlmetadata+xml");

    let xml = String::from_utf8(body).expect("valid utf8");
    assert!(xml.contains("<md:EntityDescriptor"));
    assert!(xml.contains("https://mcp.dvlpid.my.id/saml/metadata"));
    assert!(xml.contains("<md:SPSSODescriptor"));
    assert!(xml.contains("https://mcp.dvlpid.my.id/api/v1/auth/saml/acs"));
}

#[tokio::test]
async fn test_ridm_saml_login_authn_request_generation() {
    let (status, body, content_type) = SamlApiDispatcher::dispatch(
        "GET",
        "/api/v1/auth/saml/login",
        b"",
    ).await;

    assert_eq!(status, 200);
    assert_eq!(content_type, "application/json");

    let val: serde_json::Value = serde_json::from_slice(&body).expect("valid json");
    assert_eq!(val["sso_url"], "https://auth.dvlpid.my.id/t/bps/saml/sso");
    assert!(val["redirect_url"].as_str().unwrap().starts_with("https://auth.dvlpid.my.id/t/bps/saml/sso?SAMLRequest="));

    let b64 = val["saml_request"].as_str().unwrap();
    let decoded_bytes = BASE64_STANDARD.decode(b64).expect("valid base64");
    let xml = String::from_utf8(decoded_bytes).expect("valid utf8");
    assert!(xml.contains("<samlp:AuthnRequest"));
    assert!(xml.contains("Destination=\"https://auth.dvlpid.my.id/t/bps/saml/sso\""));
    assert!(xml.contains("https://mcp.dvlpid.my.id/saml/metadata</saml:Issuer>"));
}

#[tokio::test]
async fn test_ridm_saml_acs_assertion_consumption_and_jwt_issuance() {
    let mock_saml_xml = r#"
    <samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" ID="_resp_01" Version="2.0">
        <saml:Issuer xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">https://auth.dvlpid.my.id/t/bps</saml:Issuer>
        <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="_asst_01" Version="2.0">
            <saml:Issuer>https://auth.dvlpid.my.id/t/bps</saml:Issuer>
            <saml:Subject>
                <saml:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">ihzakarunia@bps.go.id</saml:NameID>
            </saml:Subject>
            <saml:Conditions NotBefore="2026-10-11T00:00:00Z" NotOnOrAfter="2026-10-12T00:00:00Z">
                <saml:AudienceRestriction>
                    <saml:Audience>https://mcp.dvlpid.my.id/saml/metadata</saml:Audience>
                </saml:AudienceRestriction>
            </saml:Conditions>
            <saml:AttributeStatement>
                <saml:Attribute Name="Department">
                    <saml:AttributeValue>BPS Mempawah / IPDS</saml:AttributeValue>
                </saml:Attribute>
                <saml:Attribute Name="Role">
                    <saml:AttributeValue>admin,developer</saml:AttributeValue>
                </saml:Attribute>
            </saml:AttributeStatement>
        </saml:Assertion>
    </samlp:Response>
    "#;

    // Test 1: JSON payload with raw XML
    let payload = serde_json::json!({
        "saml_response": mock_saml_xml
    });
    let (status, body, _) = SamlApiDispatcher::dispatch(
        "POST",
        "/api/v1/auth/saml/acs",
        &serde_json::to_vec(&payload).unwrap(),
    ).await;

    assert_eq!(status, 200);
    let val: serde_json::Value = serde_json::from_slice(&body).expect("valid json");
    assert_eq!(val["status"], "authenticated");
    assert_eq!(val["user"]["subject"], "ihzakarunia@bps.go.id");
    assert_eq!(val["user"]["department"], "BPS Mempawah / IPDS");
    assert_eq!(val["user"]["tenant"], "bps");
    assert!(val["access_token"].as_str().unwrap().starts_with("propylon_jwt_ihzakarunia@bps.go.id"));

    // Test 2: Base64 payload in JSON
    let b64_xml = BASE64_STANDARD.encode(mock_saml_xml.as_bytes());
    let payload_b64 = serde_json::json!({
        "saml_response": b64_xml
    });
    let (status_b64, body_b64, _) = SamlApiDispatcher::dispatch(
        "POST",
        "/api/v1/auth/saml/acs",
        &serde_json::to_vec(&payload_b64).unwrap(),
    ).await;
    assert_eq!(status_b64, 200);
    let val_b64: serde_json::Value = serde_json::from_slice(&body_b64).expect("valid json");
    assert_eq!(val_b64["user"]["subject"], "ihzakarunia@bps.go.id");
}

#[tokio::test]
async fn test_auth_api_dispatcher_routes_saml_seamlessly() {
    let (status, body, _) = AuthApiDispatcher::dispatch(
        "GET",
        "/api/v1/auth/saml/config",
        b"",
    ).await;

    assert_eq!(status, 200);
    let val: serde_json::Value = serde_json::from_slice(&body).expect("valid json");
    assert_eq!(val["provider"], "rIDM");
    assert_eq!(val["tenant"], "bps");
}
