// SPDX-License-Identifier: MIT

use serde_json::json;
use std::sync::Arc;

use propylon::core::transport::{JsonRpcId, JsonRpcRequest, WireProtocolHandler};
use propylon::transport::McpProtocolHandler;
use propylon::PropylonGateway;

#[tokio::test]
async fn test_mcp_web_search_advertised_and_searchable() {
    let gateway = Arc::new(PropylonGateway::default());
    let handler = McpProtocolHandler::new(gateway.clone());

    // 1. Verify gateway_web_search is advertised in tools/list
    let list_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(1)),
        method: "tools/list".to_string(),
        params: None,
    };

    let list_resp_raw = handler
        .handle_message(&serde_json::to_string(&list_req).unwrap())
        .await
        .unwrap()
        .unwrap();

    let list_resp: serde_json::Value = serde_json::from_str(&list_resp_raw).unwrap();
    let tools = list_resp["result"]["tools"].as_array().unwrap();

    let search_tool = tools
        .iter()
        .find(|t| t["name"] == "gateway_web_search")
        .expect("gateway_web_search should be in tools/list");

    assert!(search_tool["description"].as_str().unwrap().contains("Web"));
    assert_eq!(
        search_tool["inputSchema"]["properties"]["query"]["type"],
        "string"
    );

    // 2. Verify progressive discovery via gateway_search_tools finds it
    let find_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(2)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "gateway_search_tools",
            "arguments": {
                "query": "web search exa internet"
            }
        })),
    };

    let find_resp_raw = handler
        .handle_message(&serde_json::to_string(&find_req).unwrap())
        .await
        .unwrap()
        .unwrap();

    let find_resp: serde_json::Value = serde_json::from_str(&find_resp_raw).unwrap();
    let matched_tools = find_resp["result"]["tools"].as_array().unwrap();
    assert!(
        matched_tools.iter().any(|t| t["name"] == "gateway_web_search"),
        "gateway_search_tools must discover gateway_web_search"
    );

    // 3. Verify tools/call execution of gateway_web_search with added test key
    gateway.search_manager().add_key("b436b248-b64d-434d-8db6-e32dcf296a12", Some("Live Exa Key".to_string())).await;

    let call_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(3)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "gateway_web_search",
            "arguments": {
                "query": "BPS Kabupaten Mempawah",
                "limit": 1
            }
        })),
    };

    let call_resp_raw = handler
        .handle_message(&serde_json::to_string(&call_req).unwrap())
        .await
        .unwrap()
        .unwrap();

    let call_resp: serde_json::Value = serde_json::from_str(&call_resp_raw).unwrap();
    assert!(call_resp.get("result").is_some(), "Call should return result: {:?}", call_resp);
    let content = call_resp["result"]["content"].as_array().unwrap();
    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains("Web Search Results") || text.contains("Mempawah"));
}
