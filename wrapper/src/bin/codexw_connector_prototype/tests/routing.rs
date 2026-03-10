use super::super::routing::is_allowed_local_proxy_target;
use super::super::routing::resolve_proxy_target;

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
