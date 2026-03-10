use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;

use super::http::HttpRequest;
use super::routing::ProxyTarget;
use super::routing::is_allowed_local_proxy_target;
use super::routing::resolve_proxy_target;
use super::sse::wrap_event_payload;
use super::upstream::ForwardRequestError;
use super::upstream::prepare_upstream_body;
use super::upstream::supports_client_lease_injection;

#[test]
fn resolve_proxy_target_maps_http_and_sse_routes() {
    let http = resolve_proxy_target(
        "POST",
        "/v1/agents/codexw-lab/proxy/api/v1/session/new",
        "codexw-lab",
    )
    .expect("http route");
    assert_eq!(http.local_path, "/api/v1/session/new");
    assert!(!http.is_sse);
    assert!(http.session_id_hint.is_none());

    let sse = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/proxy_sse/api/v1/session/sess_1/events",
        "codexw-lab",
    )
    .expect("sse route");
    assert_eq!(sse.local_path, "/api/v1/session/sess_1/events");
    assert!(sse.is_sse);
    assert!(sse.session_id_hint.is_none());
}

#[test]
fn resolve_proxy_target_rejects_wrong_agent_for_proxy_routes() {
    assert!(
        resolve_proxy_target(
            "POST",
            "/v1/agents/other/proxy/api/v1/session/new",
            "codexw-lab",
        )
        .is_none()
    );
}

#[test]
fn allowlist_accepts_supported_http_routes() {
    let cases = [
        ("GET", "/healthz"),
        ("GET", "/api/v1/session"),
        ("GET", "/api/v1/session/sess_1"),
        ("GET", "/api/v1/session/sess_1/transcript"),
        ("GET", "/api/v1/session/sess_1/client_event"),
        ("GET", "/api/v1/session/sess_1/shells"),
        ("GET", "/api/v1/session/sess_1/shells/bg-1"),
        ("GET", "/api/v1/session/sess_1/services"),
        ("GET", "/api/v1/session/sess_1/services/dev.frontend"),
        ("GET", "/api/v1/session/sess_1/capabilities"),
        ("GET", "/api/v1/session/sess_1/capabilities/@frontend.dev"),
        ("GET", "/api/v1/session/sess_1/orchestration/status"),
        ("GET", "/api/v1/session/sess_1/orchestration/dependencies"),
        ("GET", "/api/v1/session/sess_1/orchestration/workers"),
        ("POST", "/api/v1/session/new"),
        ("POST", "/api/v1/session/attach"),
        ("POST", "/api/v1/session/client_event"),
        ("POST", "/api/v1/session/sess_1/attachment/renew"),
        ("POST", "/api/v1/session/sess_1/attachment/release"),
        ("POST", "/api/v1/session/sess_1/client_event"),
        ("POST", "/api/v1/session/sess_1/turn/start"),
        ("POST", "/api/v1/session/sess_1/turn/interrupt"),
        ("POST", "/api/v1/session/sess_1/shells/start"),
        ("POST", "/api/v1/session/sess_1/shells/bg-1/poll"),
        ("POST", "/api/v1/session/sess_1/shells/bg-1/send"),
        ("POST", "/api/v1/session/sess_1/shells/bg-1/terminate"),
        ("POST", "/api/v1/session/sess_1/services/update"),
        ("POST", "/api/v1/session/sess_1/services/bg-1/provide"),
        ("POST", "/api/v1/session/sess_1/services/bg-1/depend"),
        ("POST", "/api/v1/session/sess_1/services/bg-1/contract"),
        ("POST", "/api/v1/session/sess_1/services/bg-1/relabel"),
        ("POST", "/api/v1/session/sess_1/services/bg-1/attach"),
        ("POST", "/api/v1/session/sess_1/services/bg-1/wait"),
        ("POST", "/api/v1/session/sess_1/services/bg-1/run"),
    ];

    for (method, path) in cases {
        assert!(
            is_allowed_local_proxy_target(method, path, false),
            "expected allowlist acceptance for {method} {path}"
        );
    }
}

#[test]
fn allowlist_accepts_only_session_event_sse_route() {
    assert!(is_allowed_local_proxy_target(
        "GET",
        "/api/v1/session/sess_1/events",
        true,
    ));

    for (method, path) in [
        ("GET", "/api/v1/session/sess_1/transcript"),
        ("POST", "/api/v1/session/sess_1/events"),
        ("GET", "/api/v1/session/sess_1/services"),
    ] {
        assert!(
            !is_allowed_local_proxy_target(method, path, true),
            "expected SSE allowlist rejection for {method} {path}"
        );
    }
}

#[test]
fn allowlist_rejects_unknown_or_overbroad_proxy_routes() {
    assert!(!is_allowed_local_proxy_target(
        "DELETE",
        "/api/v1/session/sess_1/services/bg-1",
        false,
    ));
    assert!(!is_allowed_local_proxy_target(
        "GET",
        "/api/v1/session/sess_1/internal/debug",
        false,
    ));
    assert!(!is_allowed_local_proxy_target(
        "POST",
        "/api/v1/turn/start",
        false,
    ));
}

#[test]
fn client_lease_injection_support_is_limited_to_mutating_routes() {
    assert!(supports_client_lease_injection("/api/v1/session/new"));
    assert!(supports_client_lease_injection(
        "/api/v1/session/sess_1/services/bg-1/run"
    ));
    assert!(!supports_client_lease_injection(
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
fn resolve_proxy_target_maps_broker_style_session_alias_routes() {
    let cases = [
        (
            "GET",
            "/v1/agents/codexw-lab/sessions",
            "/api/v1/session",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions",
            "/api/v1/session/new",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1",
            "/api/v1/session/sess_1",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/attach",
            "/api/v1/session/attach",
            false,
            Some("sess_1"),
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/attachment/renew",
            "/api/v1/session/sess_1/attachment/renew",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/attachment/release",
            "/api/v1/session/sess_1/attachment/release",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/client-events",
            "/api/v1/session/sess_1/client_event",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/turns",
            "/api/v1/session/sess_1/turn/start",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/interrupt",
            "/api/v1/session/sess_1/turn/interrupt",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/transcript",
            "/api/v1/session/sess_1/transcript",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/events",
            "/api/v1/session/sess_1/events",
            true,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/shells",
            "/api/v1/session/sess_1/shells",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/shells",
            "/api/v1/session/sess_1/shells/start",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http",
            "/api/v1/session/sess_1/shells/@api.http",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/poll",
            "/api/v1/session/sess_1/shells/bg-2/poll",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/send",
            "/api/v1/session/sess_1/shells/bg-2/send",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/terminate",
            "/api/v1/session/sess_1/shells/bg-2/terminate",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/services",
            "/api/v1/session/sess_1/services",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.frontend",
            "/api/v1/session/sess_1/services/dev.frontend",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/provide",
            "/api/v1/session/sess_1/services/dev.api/provide",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/depend",
            "/api/v1/session/sess_1/services/dev.api/depend",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/contract",
            "/api/v1/session/sess_1/services/dev.api/contract",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/relabel",
            "/api/v1/session/sess_1/services/dev.api/relabel",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/attach",
            "/api/v1/session/sess_1/services/dev.api/attach",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/wait",
            "/api/v1/session/sess_1/services/dev.api/wait",
            false,
            None,
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run",
            "/api/v1/session/sess_1/services/dev.api/run",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/capabilities",
            "/api/v1/session/sess_1/capabilities",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/capabilities/%40frontend.dev",
            "/api/v1/session/sess_1/capabilities/@frontend.dev",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/orchestration/status",
            "/api/v1/session/sess_1/orchestration/status",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/orchestration/workers",
            "/api/v1/session/sess_1/orchestration/workers",
            false,
            None,
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/orchestration/dependencies",
            "/api/v1/session/sess_1/orchestration/dependencies",
            false,
            None,
        ),
    ];

    for (method, path, expected_local_path, expected_sse, expected_session_hint) in cases {
        let target = resolve_proxy_target(method, path, "codexw-lab")
            .unwrap_or_else(|| panic!("expected alias route mapping for {method} {path}"));
        assert_eq!(
            target.local_path, expected_local_path,
            "unexpected local target for {method} {path}"
        );
        assert_eq!(
            target.is_sse, expected_sse,
            "unexpected SSE flag for {method} {path}"
        );
        assert_eq!(
            target.session_id_hint.as_deref(),
            expected_session_hint,
            "unexpected session hint for {method} {path}"
        );
    }
}

#[test]
fn resolve_proxy_target_rejects_wrong_agent_for_alias_routes() {
    assert!(
        resolve_proxy_target("GET", "/v1/agents/other/sessions/sess_1", "codexw-lab").is_none()
    );
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

#[test]
fn wrap_event_payload_preserves_json_and_adds_broker_metadata() {
    let wrapped = wrap_event_payload(
        vec![r#"{"session_id":"sess_1","value":1}"#.to_string()],
        "codexw-lab",
        "mac-mini-01",
    );
    let json: Value = serde_json::from_str(&wrapped).expect("valid json");
    assert_eq!(json["source"], "codexw");
    assert_eq!(json["broker"]["agent_id"], "codexw-lab");
    assert_eq!(json["broker"]["deployment_id"], "mac-mini-01");
    assert_eq!(json["data"]["session_id"], "sess_1");
    assert_eq!(json["data"]["value"], 1);
}

#[test]
fn wrap_event_payload_falls_back_to_string_for_non_json_data() {
    let wrapped = wrap_event_payload(
        vec!["plain text update".to_string()],
        "codexw-lab",
        "mac-mini-01",
    );
    let json: Value = serde_json::from_str(&wrapped).expect("valid json");
    assert_eq!(json["data"], "plain text update");
}
