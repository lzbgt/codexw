use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;

use super::super::http::HttpRequest;
use super::super::routing::ProxyTarget;
use super::super::routing::supports_client_lease_injection;
use super::super::upstream::ForwardRequestError;
use super::super::upstream::prepare_upstream_body;

#[test]
fn client_lease_injection_support_is_limited_to_mutating_routes() {
    assert!(supports_client_lease_injection(
        "POST",
        "/api/v1/session/new"
    ));
    assert!(supports_client_lease_injection(
        "POST",
        "/api/v1/session/sess_1/services/bg-1/run"
    ));
    assert!(!supports_client_lease_injection(
        "GET",
        "/api/v1/session/new"
    ));
    assert!(!supports_client_lease_injection(
        "GET",
        "/api/v1/session/sess_1/transcript"
    ));
}

#[test]
fn prepare_upstream_body_injects_client_and_lease_headers_into_empty_json_body() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/v1/agents/codexw-lab/proxy/api/v1/session/new".to_string(),
        headers: HashMap::from([
            ("x-codexw-client-id".to_string(), "mobile-ios".to_string()),
            ("x-codexw-lease-seconds".to_string(), "30".to_string()),
        ]),
        body: Vec::new(),
    };
    let (content_type, body) = prepare_upstream_body(
        &request,
        &ProxyTarget {
            local_path: "/api/v1/session/new".to_string(),
            is_sse: false,
            session_id_hint: None,
        },
    )
    .expect("prepared body");
    assert_eq!(content_type.as_deref(), Some("application/json"));
    let json: Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(json["client_id"], "mobile-ios");
    assert_eq!(json["lease_seconds"], 30);
}

#[test]
fn prepare_upstream_body_merges_headers_without_overwriting_explicit_fields() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/v1/agents/codexw-lab/proxy/api/v1/session/attach".to_string(),
        headers: HashMap::from([
            ("content-type".to_string(), "application/json".to_string()),
            ("x-codexw-client-id".to_string(), "mobile-ios".to_string()),
            ("x-codexw-lease-seconds".to_string(), "45".to_string()),
        ]),
        body: serde_json::to_vec(&json!({
            "session_id": "sess_1",
            "thread_id": "thread_1",
            "client_id": "webui",
            "lease_seconds": 90
        }))
        .expect("serialize"),
    };
    let (_, body) = prepare_upstream_body(
        &request,
        &ProxyTarget {
            local_path: "/api/v1/session/attach".to_string(),
            is_sse: false,
            session_id_hint: None,
        },
    )
    .expect("prepared body");
    let json: Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(json["client_id"], "webui");
    assert_eq!(json["lease_seconds"], 90);
}

#[test]
fn prepare_upstream_body_rejects_invalid_lease_header() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/v1/agents/codexw-lab/proxy/api/v1/session/new".to_string(),
        headers: HashMap::from([(
            "x-codexw-lease-seconds".to_string(),
            "not-a-number".to_string(),
        )]),
        body: Vec::new(),
    };
    let err = prepare_upstream_body(
        &request,
        &ProxyTarget {
            local_path: "/api/v1/session/new".to_string(),
            is_sse: false,
            session_id_hint: None,
        },
    )
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
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/v1/agents/codexw-lab/proxy/api/v1/session/new".to_string(),
        headers: HashMap::from([("x-codexw-client-id".to_string(), "mobile-ios".to_string())]),
        body: serde_json::to_vec(&json!(["not", "an", "object"])).expect("serialize"),
    };
    let err = prepare_upstream_body(
        &request,
        &ProxyTarget {
            local_path: "/api/v1/session/new".to_string(),
            is_sse: false,
            session_id_hint: None,
        },
    )
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
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/v1/agents/codexw-lab/sessions/sess_1/attach".to_string(),
        headers: HashMap::new(),
        body: serde_json::to_vec(&json!({
            "thread_id": "thread_1"
        }))
        .expect("serialize"),
    };
    let (_, body) = prepare_upstream_body(
        &request,
        &ProxyTarget {
            local_path: "/api/v1/session/attach".to_string(),
            is_sse: false,
            session_id_hint: Some("sess_1".to_string()),
        },
    )
    .expect("prepared body");
    let json: Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(json["session_id"], "sess_1");
    assert_eq!(json["thread_id"], "thread_1");
}
