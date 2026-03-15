use crate::routing::resolve_proxy_target;

#[derive(Clone, Copy)]
struct AliasRouteCase {
    method: &'static str,
    path: &'static str,
    local_path: &'static str,
    is_sse: bool,
    session_id_hint: Option<&'static str>,
}

const READ_ALIAS_ROUTES: &[AliasRouteCase] = &[
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions",
        local_path: "/api/v1/session",
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
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/transcript",
        local_path: "/api/v1/session/sess_1/transcript",
        is_sse: false,
        session_id_hint: None,
    },
];

#[test]
fn resolve_proxy_target_maps_session_read_alias_routes() {
    for case in READ_ALIAS_ROUTES {
        let target =
            resolve_proxy_target(case.method, case.path, "codexw-lab").unwrap_or_else(|| {
                panic!(
                    "expected session read alias route mapping for {} {}",
                    case.method, case.path
                )
            });
        assert_eq!(target.local_path, case.local_path);
        assert_eq!(target.is_sse, case.is_sse);
        assert_eq!(target.session_id_hint.as_deref(), case.session_id_hint);
    }
}

#[test]
fn resolve_proxy_target_rejects_wrong_agent_for_session_read_alias_routes() {
    assert!(
        resolve_proxy_target("GET", "/v1/agents/other/sessions/sess_1", "codexw-lab").is_none()
    );
}

#[test]
fn resolve_proxy_target_rejects_wrong_methods_for_session_read_alias_routes() {
    for (method, path) in [
        ("POST", "/v1/agents/codexw-lab/sessions/sess_1"),
        ("POST", "/v1/agents/codexw-lab/sessions/sess_1/transcript"),
        ("DELETE", "/v1/agents/codexw-lab/sessions"),
    ] {
        assert!(
            resolve_proxy_target(method, path, "codexw-lab").is_none(),
            "expected session read alias route to reject wrong method for {method} {path}"
        );
    }
}
