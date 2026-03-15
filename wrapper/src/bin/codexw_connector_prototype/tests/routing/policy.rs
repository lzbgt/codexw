use crate::routing::is_allowed_local_proxy_target;
use crate::routing::resolve_proxy_target;
use crate::routing::supports_client_lease_injection;

const ALLOWED_HTTP_ROUTES: &[(&str, &str)] = &[
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
    ("POST", "/api/v1/turn/start"),
    ("POST", "/api/v1/turn/interrupt"),
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

#[test]
fn allowlist_accepts_supported_http_routes() {
    for (method, path) in ALLOWED_HTTP_ROUTES {
        assert!(
            is_allowed_local_proxy_target(method, path, false),
            "expected allowlist acceptance for {method} {path}"
        );
    }
}

#[test]
fn supported_post_routes_remain_client_lease_injection_eligible() {
    for (method, path) in ALLOWED_HTTP_ROUTES {
        let expected = *method == "POST";
        assert_eq!(
            supports_client_lease_injection(method, path),
            expected,
            "unexpected client/lease injection eligibility for {method} {path}"
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
        "GET",
        "/api/v1/turn/start",
        false,
    ));
}

#[test]
fn broker_alias_routes_resolve_only_to_allowlisted_local_targets() {
    for (method, path) in [
        ("GET", "/v1/agents/codexw-lab/sessions"),
        ("POST", "/v1/agents/codexw-lab/sessions"),
        ("GET", "/v1/agents/codexw-lab/sessions/sess_1/events"),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run",
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http/poll",
        ),
    ] {
        let target = resolve_proxy_target(method, path, "codexw-lab")
            .unwrap_or_else(|| panic!("expected alias route mapping for {method} {path}"));
        assert!(
            is_allowed_local_proxy_target(method, &target.local_path, target.is_sse),
            "expected resolved alias target to stay allowlisted for {method} {path} -> {}",
            target.local_path
        );
    }
}
