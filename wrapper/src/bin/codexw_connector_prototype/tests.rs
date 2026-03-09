use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;

use super::http::HttpRequest;
use super::routing::ProxyTarget;
use super::routing::is_allowed_local_proxy_target;
use super::routing::resolve_proxy_target;
use super::sse::wrap_event_payload;
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
    assert!(is_allowed_local_proxy_target(
        "POST",
        "/api/v1/session/new",
        false,
    ));
    assert!(is_allowed_local_proxy_target(
        "GET",
        "/api/v1/session/sess_1/orchestration/workers",
        false,
    ));
    assert!(is_allowed_local_proxy_target(
        "POST",
        "/api/v1/session/sess_1/services/bg-1/run",
        false,
    ));
    assert!(is_allowed_local_proxy_target(
        "GET",
        "/api/v1/session/sess_1/services/dev.frontend",
        false,
    ));
    assert!(is_allowed_local_proxy_target(
        "GET",
        "/api/v1/session/sess_1/shells/dev.api",
        false,
    ));
    assert!(is_allowed_local_proxy_target(
        "GET",
        "/api/v1/session/sess_1/capabilities/@frontend.dev",
        false,
    ));
}

#[test]
fn allowlist_accepts_only_session_event_sse_route() {
    assert!(is_allowed_local_proxy_target(
        "GET",
        "/api/v1/session/sess_1/events",
        true,
    ));
    assert!(!is_allowed_local_proxy_target(
        "GET",
        "/api/v1/session/sess_1/transcript",
        true,
    ));
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
    assert!(format!("{err:#}").contains("x-codexw-lease-seconds"));
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
    assert!(format!("{err:#}").contains("JSON object body"));
}

#[test]
fn resolve_proxy_target_maps_broker_style_session_alias_routes() {
    let list = resolve_proxy_target("GET", "/v1/agents/codexw-lab/sessions", "codexw-lab")
        .expect("list route");
    assert_eq!(list.local_path, "/api/v1/session");
    assert!(!list.is_sse);
    assert!(list.session_id_hint.is_none());

    let create = resolve_proxy_target("POST", "/v1/agents/codexw-lab/sessions", "codexw-lab")
        .expect("create route");
    assert_eq!(create.local_path, "/api/v1/session/new");
    assert!(!create.is_sse);
    assert!(create.session_id_hint.is_none());

    let inspect =
        resolve_proxy_target("GET", "/v1/agents/codexw-lab/sessions/sess_1", "codexw-lab")
            .expect("inspect route");
    assert_eq!(inspect.local_path, "/api/v1/session/sess_1");

    let attach = resolve_proxy_target(
        "POST",
        "/v1/agents/codexw-lab/sessions/sess_1/attach",
        "codexw-lab",
    )
    .expect("attach route");
    assert_eq!(attach.local_path, "/api/v1/session/attach");
    assert_eq!(attach.session_id_hint.as_deref(), Some("sess_1"));

    let shell_detail = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http",
        "codexw-lab",
    )
    .expect("shell detail route");
    assert_eq!(
        shell_detail.local_path,
        "/api/v1/session/sess_1/shells/@api.http"
    );
    assert!(shell_detail.session_id_hint.is_none());

    let renew = resolve_proxy_target(
        "POST",
        "/v1/agents/codexw-lab/sessions/sess_1/attachment/renew",
        "codexw-lab",
    )
    .expect("renew route");
    assert_eq!(renew.local_path, "/api/v1/session/sess_1/attachment/renew");

    let release = resolve_proxy_target(
        "POST",
        "/v1/agents/codexw-lab/sessions/sess_1/attachment/release",
        "codexw-lab",
    )
    .expect("release route");
    assert_eq!(
        release.local_path,
        "/api/v1/session/sess_1/attachment/release"
    );

    let turns = resolve_proxy_target(
        "POST",
        "/v1/agents/codexw-lab/sessions/sess_1/turns",
        "codexw-lab",
    )
    .expect("turn route");
    assert_eq!(turns.local_path, "/api/v1/session/sess_1/turn/start");

    let transcript = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/sessions/sess_1/transcript",
        "codexw-lab",
    )
    .expect("transcript route");
    assert_eq!(transcript.local_path, "/api/v1/session/sess_1/transcript");

    let events = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/sessions/sess_1/events",
        "codexw-lab",
    )
    .expect("events route");
    assert_eq!(events.local_path, "/api/v1/session/sess_1/events");
    assert!(events.is_sse);

    let shell_list = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/sessions/sess_1/shells",
        "codexw-lab",
    )
    .expect("shell list route");
    assert_eq!(shell_list.local_path, "/api/v1/session/sess_1/shells");

    let shell_start = resolve_proxy_target(
        "POST",
        "/v1/agents/codexw-lab/sessions/sess_1/shells",
        "codexw-lab",
    )
    .expect("shell start route");
    assert_eq!(
        shell_start.local_path,
        "/api/v1/session/sess_1/shells/start"
    );

    let shell_send = resolve_proxy_target(
        "POST",
        "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/send",
        "codexw-lab",
    )
    .expect("shell send route");
    assert_eq!(
        shell_send.local_path,
        "/api/v1/session/sess_1/shells/bg-2/send"
    );

    let services = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/sessions/sess_1/services",
        "codexw-lab",
    )
    .expect("services route");
    assert_eq!(services.local_path, "/api/v1/session/sess_1/services");

    let service_detail = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/sessions/sess_1/services/dev.frontend",
        "codexw-lab",
    )
    .expect("service detail route");
    assert_eq!(
        service_detail.local_path,
        "/api/v1/session/sess_1/services/dev.frontend"
    );

    let capabilities = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/sessions/sess_1/capabilities",
        "codexw-lab",
    )
    .expect("capabilities route");
    assert_eq!(
        capabilities.local_path,
        "/api/v1/session/sess_1/capabilities"
    );

    let capability_detail = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/sessions/sess_1/capabilities/%40frontend.dev",
        "codexw-lab",
    )
    .expect("capability detail route");
    assert_eq!(
        capability_detail.local_path,
        "/api/v1/session/sess_1/capabilities/@frontend.dev"
    );

    let service_run = resolve_proxy_target(
        "POST",
        "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run",
        "codexw-lab",
    )
    .expect("service run route");
    assert_eq!(
        service_run.local_path,
        "/api/v1/session/sess_1/services/dev.api/run"
    );
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
