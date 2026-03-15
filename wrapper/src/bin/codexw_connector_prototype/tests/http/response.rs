use serde_json::Value;
use std::collections::HashMap;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;

use super::super::super::Cli;
use crate::http::from_upstream_response;
use crate::http::json_error_response;
use crate::http::json_ok_response;
use crate::upstream::UpstreamResponse;

fn sample_cli() -> Cli {
    Cli {
        bind: "127.0.0.1:0".to_string(),
        local_api_base: "http://127.0.0.1:8080".to_string(),
        local_api_token: Some("secret".to_string()),
        connector_token: Some("connector".to_string()),
        agent_id: "codexw-lab".to_string(),
        deployment_id: "mac-mini-01".to_string(),
    }
}

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
fn from_upstream_response_adds_adapter_header_and_forwards_local_api_header() {
    let response = from_upstream_response(
        UpstreamResponse {
            status: 200,
            reason: "OK".to_string(),
            headers: HashMap::from([
                ("content-type".to_string(), "application/json".to_string()),
                (
                    "x-codexw-local-api-version".to_string(),
                    CODEXW_LOCAL_API_VERSION.to_string(),
                ),
            ]),
            body: br#"{"ok":true}"#.to_vec(),
        },
        &sample_cli(),
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Codexw-Agent-Id".to_string(), "codexw-lab".to_string()),
            (
                "X-Codexw-Deployment-Id".to_string(),
                "mac-mini-01".to_string()
            ),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string()
            ),
            (
                HEADER_LOCAL_API_VERSION.to_string(),
                CODEXW_LOCAL_API_VERSION.to_string()
            ),
        ]
    );
    let body: Value = serde_json::from_slice(&response.body).expect("json body");
    assert_eq!(body["ok"], true);
}
