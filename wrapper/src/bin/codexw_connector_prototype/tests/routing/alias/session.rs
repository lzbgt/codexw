use crate::routing::resolve_proxy_target;

#[derive(Clone, Copy)]
struct AliasRouteCase {
    method: &'static str,
    path: &'static str,
    local_path: &'static str,
    is_sse: bool,
    session_id_hint: Option<&'static str>,
}

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

#[test]
fn resolve_proxy_target_rejects_wrong_methods_for_write_alias_routes() {
    for (method, path) in [
        ("GET", "/v1/agents/codexw-lab/sessions/sess_1/attach"),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/attachment/renew",
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/attachment/release",
        ),
        ("GET", "/v1/agents/codexw-lab/sessions/sess_1/client-events"),
        ("GET", "/v1/agents/codexw-lab/sessions/sess_1/turns"),
        ("GET", "/v1/agents/codexw-lab/sessions/sess_1/interrupt"),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http/send",
        ),
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run",
        ),
        ("DELETE", "/v1/agents/codexw-lab/sessions"),
        ("DELETE", "/v1/agents/codexw-lab/sessions/sess_1/shells"),
    ] {
        assert!(
            resolve_proxy_target(method, path, "codexw-lab").is_none(),
            "expected write alias route to reject wrong method for {method} {path}"
        );
    }
}
