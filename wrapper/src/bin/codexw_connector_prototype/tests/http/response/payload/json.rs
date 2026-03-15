use serde_json::Value;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_BROKER_ADAPTER_VERSION;

use crate::http::json_error_response;
use crate::http::json_ok_response;

#[test]
fn json_ok_response_adds_adapter_version_header_and_body() {
    let response = json_ok_response(serde_json::json!({
        "ok": true,
        "session_id": "sess_1"
    }));
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string()
            ),
        ]
    );
    let body: Value = serde_json::from_slice(&response.body).expect("json body");
    assert_eq!(
        body["broker_adapter_version"],
        CODEXW_BROKER_ADAPTER_VERSION
    );
    assert_eq!(body["session_id"], "sess_1");
}

#[test]
fn json_error_response_adds_adapter_version_header_and_body() {
    let response = json_error_response(
        400,
        "validation_error",
        "bad lease header",
        Some(serde_json::json!({
            "field": "x-codexw-lease-seconds"
        })),
    );
    assert_eq!(response.status, 400);
    assert_eq!(
        response.headers,
        vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string()
            ),
        ]
    );
    let body: Value = serde_json::from_slice(&response.body).expect("json body");
    assert_eq!(body["ok"], false);
    assert_eq!(
        body["broker_adapter_version"],
        CODEXW_BROKER_ADAPTER_VERSION
    );
    assert_eq!(body["error"]["code"], "validation_error");
    assert_eq!(body["error"]["details"]["field"], "x-codexw-lease-seconds");
}

#[test]
fn json_error_response_uses_forbidden_reason_for_403() {
    let response = json_error_response(403, "route_not_allowed", "outside allowlist", None);

    assert_eq!(response.status, 403);
    assert_eq!(response.reason, "Forbidden");
}
