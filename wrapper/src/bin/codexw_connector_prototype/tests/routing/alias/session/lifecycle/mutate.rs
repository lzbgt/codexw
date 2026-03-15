use crate::routing::resolve_proxy_target;

#[derive(Clone, Copy)]
struct AliasRouteCase {
    method: &'static str,
    path: &'static str,
    local_path: &'static str,
    session_id_hint: Option<&'static str>,
}

const MUTATE_ALIAS_ROUTES: &[AliasRouteCase] = &[
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions",
        local_path: "/api/v1/session/new",
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/attach",
        local_path: "/api/v1/session/attach",
        session_id_hint: Some("sess_1"),
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/attachment/renew",
        local_path: "/api/v1/session/sess_1/attachment/renew",
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/attachment/release",
        local_path: "/api/v1/session/sess_1/attachment/release",
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/client-events",
        local_path: "/api/v1/session/sess_1/client_event",
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/turns",
        local_path: "/api/v1/session/sess_1/turn/start",
        session_id_hint: None,
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/interrupt",
        local_path: "/api/v1/session/sess_1/turn/interrupt",
        session_id_hint: None,
    },
];

#[test]
fn resolve_proxy_target_maps_session_mutation_alias_routes() {
    for case in MUTATE_ALIAS_ROUTES {
        let target =
            resolve_proxy_target(case.method, case.path, "codexw-lab").unwrap_or_else(|| {
                panic!(
                    "expected session mutation alias route mapping for {} {}",
                    case.method, case.path
                )
            });
        assert_eq!(target.local_path, case.local_path);
        assert!(!target.is_sse);
        assert_eq!(target.session_id_hint.as_deref(), case.session_id_hint);
    }
}

#[test]
fn resolve_proxy_target_rejects_wrong_methods_for_session_mutation_alias_routes() {
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
    ] {
        assert!(
            resolve_proxy_target(method, path, "codexw-lab").is_none(),
            "expected session mutation alias route to reject wrong method for {method} {path}"
        );
    }
}
