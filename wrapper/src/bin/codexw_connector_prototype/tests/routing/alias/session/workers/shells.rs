use crate::routing::resolve_proxy_target;

#[derive(Clone, Copy)]
struct AliasRouteCase {
    method: &'static str,
    path: &'static str,
    local_path: &'static str,
}

const SHELL_ALIAS_ROUTES: &[AliasRouteCase] = &[
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells",
        local_path: "/api/v1/session/sess_1/shells",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells",
        local_path: "/api/v1/session/sess_1/shells/start",
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http",
        local_path: "/api/v1/session/sess_1/shells/@api.http",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/poll",
        local_path: "/api/v1/session/sess_1/shells/bg-2/poll",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http/poll",
        local_path: "/api/v1/session/sess_1/shells/@api.http/poll",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/send",
        local_path: "/api/v1/session/sess_1/shells/bg-2/send",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/shells/bg-2/terminate",
        local_path: "/api/v1/session/sess_1/shells/bg-2/terminate",
    },
];

#[test]
fn resolve_proxy_target_maps_shell_alias_routes() {
    for case in SHELL_ALIAS_ROUTES {
        let target =
            resolve_proxy_target(case.method, case.path, "codexw-lab").unwrap_or_else(|| {
                panic!(
                    "expected shell alias route mapping for {} {}",
                    case.method, case.path
                )
            });
        assert_eq!(target.local_path, case.local_path);
        assert!(!target.is_sse);
        assert!(target.session_id_hint.is_none());
    }
}

#[test]
fn resolve_proxy_target_rejects_invalid_percent_encoded_shell_alias_segments() {
    for (method, path) in [
        ("GET", "/v1/agents/codexw-lab/sessions/sess_1/shells/%ZZ"),
        (
            "POST",
            "/v1/agents/codexw-lab/sessions/sess_1/shells/%ZZ/poll",
        ),
    ] {
        assert!(
            resolve_proxy_target(method, path, "codexw-lab").is_none(),
            "expected invalid percent-encoded shell alias segment to be rejected for {method} {path}"
        );
    }
}

#[test]
fn resolve_proxy_target_rejects_wrong_methods_for_shell_alias_routes() {
    for (method, path) in [
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http/send",
        ),
        ("DELETE", "/v1/agents/codexw-lab/sessions/sess_1/shells"),
    ] {
        assert!(
            resolve_proxy_target(method, path, "codexw-lab").is_none(),
            "expected shell alias route to reject wrong method for {method} {path}"
        );
    }
}
