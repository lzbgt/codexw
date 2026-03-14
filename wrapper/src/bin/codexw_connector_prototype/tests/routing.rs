use super::super::routing::is_allowed_local_proxy_target;
use super::super::routing::resolve_proxy_target;
use super::super::routing::supports_client_lease_injection;

#[derive(Clone, Copy)]
struct AliasRouteCase {
    method: &'static str,
    path: &'static str,
    local_path: &'static str,
    is_sse: bool,
    session_id_hint: Option<&'static str>,
}

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

const BROKER_ALIAS_ROUTES: &[AliasRouteCase] = &[
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions",
        local_path: "/api/v1/session",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions",
        local_path: "/api/v1/session/new",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1",
        local_path: "/api/v1/session/sess_1",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/attach",
        local_path: "/api/v1/session/attach",
        is_sse: false,
        session_id_hint: Some("sess_1"),
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/attachment/renew",
        local_path: "/api/v1/session/sess_1/attachment/renew",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/attachment/release",
        local_path: "/api/v1/session/sess_1/attachment/release",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/client-events",
        local_path: "/api/v1/session/sess_1/client_event",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/turns",
        local_path: "/api/v1/session/sess_1/turn/start",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/interrupt",
        local_path: "/api/v1/session/sess_1/turn/interrupt",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/transcript",
        local_path: "/api/v1/session/sess_1/transcript",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/events",
        local_path: "/api/v1/session/sess_1/events",
        is_sse: true,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells",
        local_path: "/api/v1/session/sess_1/shells",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells",
        local_path: "/api/v1/session/sess_1/shells/start",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http",
        local_path: "/api/v1/session/sess_1/shells/@api.http",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/poll",
        local_path: "/api/v1/session/sess_1/shells/bg-2/poll",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http/poll",
        local_path: "/api/v1/session/sess_1/shells/@api.http/poll",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/send",
        local_path: "/api/v1/session/sess_1/shells/bg-2/send",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/terminate",
        local_path: "/api/v1/session/sess_1/shells/bg-2/terminate",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services",
        local_path: "/api/v1/session/sess_1/services",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.frontend",
        local_path: "/api/v1/session/sess_1/services/dev.frontend",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/provide",
        local_path: "/api/v1/session/sess_1/services/dev.api/provide",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/depend",
        local_path: "/api/v1/session/sess_1/services/dev.api/depend",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/contract",
        local_path: "/api/v1/session/sess_1/services/dev.api/contract",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/relabel",
        local_path: "/api/v1/session/sess_1/services/dev.api/relabel",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/attach",
        local_path: "/api/v1/session/sess_1/services/dev.api/attach",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/%40frontend.dev/attach",
        local_path: "/api/v1/session/sess_1/services/@frontend.dev/attach",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/wait",
        local_path: "/api/v1/session/sess_1/services/dev.api/wait",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run",
        local_path: "/api/v1/session/sess_1/services/dev.api/run",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/capabilities",
        local_path: "/api/v1/session/sess_1/capabilities",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/capabilities/%40frontend.dev",
        local_path: "/api/v1/session/sess_1/capabilities/@frontend.dev",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/orchestration/status",
        local_path: "/api/v1/session/sess_1/orchestration/status",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/orchestration/workers",
        local_path: "/api/v1/session/sess_1/orchestration/workers",
        is_sse: false,
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/orchestration/dependencies",
        local_path: "/api/v1/session/sess_1/orchestration/dependencies",
        is_sse: false,
        session_id_hint: None,
    },
];

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
        "POST",
        "/api/v1/turn/start",
        false,
    ));
}

#[test]
fn resolve_proxy_target_maps_broker_style_session_alias_routes() {
    for case in BROKER_ALIAS_ROUTES {
        let target =
            resolve_proxy_target(case.method, case.path, "codexw-lab").unwrap_or_else(|| {
                panic!(
                    "expected alias route mapping for {} {}",
                    case.method, case.path
                )
            });
        assert_eq!(
            target.local_path, case.local_path,
            "unexpected local target for {} {}",
            case.method, case.path
        );
        assert_eq!(
            target.is_sse, case.is_sse,
            "unexpected SSE flag for {} {}",
            case.method, case.path
        );
        assert_eq!(
            target.session_id_hint.as_deref(),
            case.session_id_hint,
            "unexpected session hint for {} {}",
            case.method,
            case.path
        );
    }
}

#[test]
fn broker_alias_routes_resolve_only_to_allowlisted_local_targets() {
    for case in BROKER_ALIAS_ROUTES {
        let target =
            resolve_proxy_target(case.method, case.path, "codexw-lab").unwrap_or_else(|| {
                panic!(
                    "expected alias route mapping for {} {}",
                    case.method, case.path
                )
            });
        assert!(
            is_allowed_local_proxy_target(case.method, &target.local_path, target.is_sse),
            "expected resolved alias target to stay allowlisted for {} {} -> {}",
            case.method,
            case.path,
            target.local_path
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
fn resolve_proxy_target_rejects_invalid_percent_encoded_alias_segments() {
    for (method, path) in [
        ("GET", "/v1/agents/codexw-lab/sessions/sess_1/shells/%ZZ"),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/shells/%ZZ/poll",
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/capabilities/%ZZ",
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/services/%ZZ/attach",
        ),
    ] {
        assert!(
            resolve_proxy_target(method, path, "codexw-lab").is_none(),
            "expected invalid percent-encoded alias segment to be rejected for {method} {path}"
        );
    }
}

#[test]
fn resolve_proxy_target_rejects_wrong_methods_for_read_only_alias_routes() {
    for (method, path) in [
        ("POST", "/v1/agents/codexw-lab/sessions/sess_1"),
        ("POST", "/v1/agents/codexw-lab/sessions/sess_1/transcript"),
        ("POST", "/v1/agents/codexw-lab/sessions/sess_1/services"),
        ("POST", "/v1/agents/codexw-lab/sessions/sess_1/capabilities"),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/orchestration/status",
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/orchestration/workers",
        ),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/orchestration/dependencies",
        ),
    ] {
        assert!(
            resolve_proxy_target(method, path, "codexw-lab").is_none(),
            "expected read-only alias route to reject wrong method for {method} {path}"
        );
    }
}
