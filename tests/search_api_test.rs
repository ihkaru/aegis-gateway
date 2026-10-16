// SPDX-License-Identifier: MIT

use serde_json::json;
use std::sync::Arc;

use propylon::control::SearchApiDispatcher;
use propylon::PropylonGateway;

#[tokio::test]
async fn test_search_api_crud_lifecycle() {
    let gateway = Arc::new(PropylonGateway::default());

    // 1. GET initial keys list
    let (status, body, _) = SearchApiDispatcher::dispatch(
        "GET",
        "/api/v1/search/keys",
        &[],
        Some(&gateway),
    )
    .await;
    assert_eq!(status, 200);
    let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["provider"], "exa");
    assert_eq!(val["strategy"], "round_robin");

    // 2. POST add key
    let add_payload = json!({
        "key": "test-key-secret-1234567890",
        "label": "Playground Key #1"
    });
    let (status, body, _) = SearchApiDispatcher::dispatch(
        "POST",
        "/api/v1/search/keys",
        &serde_json::to_vec(&add_payload).unwrap(),
        Some(&gateway),
    )
    .await;
    assert_eq!(status, 201);
    let add_val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(add_val["status"], "created");
    let key_id = add_val["id"].as_str().unwrap().to_string();

    // 3. GET verify key appears in list and is masked
    let (status, body, _) = SearchApiDispatcher::dispatch(
        "GET",
        "/api/v1/search/keys",
        &[],
        Some(&gateway),
    )
    .await;
    assert_eq!(status, 200);
    let list_val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let keys = list_val["keys"].as_array().unwrap();
    let found = keys.iter().find(|k| k["id"] == key_id).unwrap();
    assert_eq!(found["label"], "Playground Key #1");
    assert!(found["active"].as_bool().unwrap());
    assert_eq!(found["masked_key"], "test...7890");

    // 4. PUT toggle key to inactive
    let toggle_payload = json!({ "active": false });
    let (status, body, _) = SearchApiDispatcher::dispatch(
        "PUT",
        &format!("/api/v1/search/keys/{key_id}/toggle"),
        &serde_json::to_vec(&toggle_payload).unwrap(),
        Some(&gateway),
    )
    .await;
    assert_eq!(status, 200);
    let toggle_val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!toggle_val["active"].as_bool().unwrap());

    // 5. PUT change strategy to least_used
    let strat_payload = json!({ "strategy": "least_used" });
    let (status, body, _) = SearchApiDispatcher::dispatch(
        "PUT",
        "/api/v1/search/strategy",
        &serde_json::to_vec(&strat_payload).unwrap(),
        Some(&gateway),
    )
    .await;
    assert_eq!(status, 200);
    let strat_val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(strat_val["strategy"], "least_used");

    // 6. DELETE revoke key
    let (status, body, _) = SearchApiDispatcher::dispatch(
        "DELETE",
        &format!("/api/v1/search/keys/{key_id}"),
        &[],
        Some(&gateway),
    )
    .await;
    assert_eq!(status, 200);
    let del_val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(del_val["status"], "revoked");

    // 7. GET verify key is gone
    let (status, body, _) = SearchApiDispatcher::dispatch(
        "GET",
        "/api/v1/search/keys",
        &[],
        Some(&gateway),
    )
    .await;
    assert_eq!(status, 200);
    let final_list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let final_keys = final_list["keys"].as_array().unwrap();
    assert!(!final_keys.iter().any(|k| k["id"] == key_id));
}
