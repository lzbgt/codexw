use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;

use crate::upstream::ForwardRequestError;
use crate::upstream::prepare_upstream_body;

use super::request_with;
use super::target_with;

#[test]
fn prepare_upstream_body_injects_client_and_lease_headers_into_empty_json_body() {
    let request = request_with(
        "POST",
        "/v1/agents/codexw-lab/proxy/api/v1/session/new",
        HashMap::from([
            ("x-codexw-client-id".to_string(), "mobile-ios".to_string()),
            ("x-codexw-lease-seconds".to_string(), "30".to_string()),
        ]),
        Vec::new(),
    );

    let (content_type, body) =
        prepare_upstream_body(&request, &target_with("/api/v1/session/new", None))
            .expect("prepared body");
    assert_eq!(content_type.as_deref(), Some("application/json"));
    let json: Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(json["client_id"], "mobile-ios");
    assert_eq!(json["lease_seconds"], 30);
}

#[test]
fn prepare_upstream_body_merges_headers_without_overwriting_explicit_fields() {
    let request = request_with(
        "POST",
        "/v1/agents/codexw-lab/proxy/api/v1/session/attach",
        HashMap::from([
            ("content-type".to_string(), "application/json".to_string()),
            ("x-codexw-client-id".to_string(), "mobile-ios".to_string()),
            ("x-codexw-lease-seconds".to_string(), "45".to_string()),
        ]),
        serde_json::to_vec(&json!({
            "session_id": "sess_1",
            "thread_id": "thread_1",
            "client_id": "webui",
            "lease_seconds": 90
        }))
        .expect("serialize"),
    );

    let (_, body) = prepare_upstream_body(&request, &target_with("/api/v1/session/attach", None))
        .expect("prepared body");
    let json: Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(json["client_id"], "webui");
    assert_eq!(json["lease_seconds"], 90);
}

#[test]
fn prepare_upstream_body_rejects_invalid_lease_header() {
    let request = request_with(
        "POST",
        "/v1/agents/codexw-lab/proxy/api/v1/session/new",
        HashMap::from([(
            "x-codexw-lease-seconds".to_string(),
            "not-a-number".to_string(),
        )]),
        Vec::new(),
    );

    let err = prepare_upstream_body(&request, &target_with("/api/v1/session/new", None))
        .expect_err("invalid lease");
    match err {
        ForwardRequestError::Validation { message, details } => {
            assert!(message.contains("x-codexw-lease-seconds"));
            assert_eq!(details.expect("details")["field"], "x-codexw-lease-seconds");
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn prepare_upstream_body_rejects_non_object_json_when_injecting() {
    let request = request_with(
        "POST",
        "/v1/agents/codexw-lab/proxy/api/v1/session/new",
        HashMap::from([("x-codexw-client-id".to_string(), "mobile-ios".to_string())]),
        serde_json::to_vec(&json!(["not", "an", "object"])).expect("serialize"),
    );

    let err = prepare_upstream_body(&request, &target_with("/api/v1/session/new", None))
        .expect_err("invalid body");
    match err {
        ForwardRequestError::Validation { message, details } => {
            assert!(message.contains("JSON object body"));
            assert_eq!(details.expect("details")["field"], "body");
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn prepare_upstream_body_injects_session_id_hint_for_attach_alias() {
    let request = request_with(
        "POST",
        "/v1/agents/codexw-lab/sessions/sess_1/attach",
        HashMap::new(),
        serde_json::to_vec(&json!({
            "thread_id": "thread_1"
        }))
        .expect("serialize"),
    );

    let (_, body) = prepare_upstream_body(
        &request,
        &target_with("/api/v1/session/attach", Some("sess_1")),
    )
    .expect("prepared body");
    let json: Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(json["session_id"], "sess_1");
    assert_eq!(json["thread_id"], "thread_1");
}
