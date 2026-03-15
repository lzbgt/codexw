use crate::routing::resolve_proxy_target;

#[derive(Clone, Copy)]
struct AliasRouteCase {
    method: &'static str,
    path: &'static str,
    local_path: &'static str,
}

const SERVICE_ALIAS_ROUTES: &[AliasRouteCase] = &[
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services",
        local_path: "/api/v1/session/sess_1/services",
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.frontend",
        local_path: "/api/v1/session/sess_1/services/dev.frontend",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/provide",
        local_path: "/api/v1/session/sess_1/services/dev.api/provide",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/depend",
        local_path: "/api/v1/session/sess_1/services/dev.api/depend",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/contract",
        local_path: "/api/v1/session/sess_1/services/dev.api/contract",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/relabel",
        local_path: "/api/v1/session/sess_1/services/dev.api/relabel",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/attach",
        local_path: "/api/v1/session/sess_1/services/dev.api/attach",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/%40frontend.dev/attach",
        local_path: "/api/v1/session/sess_1/services/@frontend.dev/attach",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/wait",
        local_path: "/api/v1/session/sess_1/services/dev.api/wait",
    },
    AliasRouteCase {
        method: "POST",
        path: "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run",
        local_path: "/api/v1/session/sess_1/services/dev.api/run",
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/capabilities",
        local_path: "/api/v1/session/sess_1/capabilities",
    },
    AliasRouteCase {
        method: "GET",
        path: "/v1/agents/codexw-lab/sessions/sess_1/capabilities/%40frontend.dev",
        local_path: "/api/v1/session/sess_1/capabilities/@frontend.dev",
    },
];

#[test]
fn resolve_proxy_target_maps_service_and_capability_alias_routes() {
    for case in SERVICE_ALIAS_ROUTES {
        let target =
            resolve_proxy_target(case.method, case.path, "codexw-lab").unwrap_or_else(|| {
                panic!(
                    "expected service/capability alias route mapping for {} {}",
                    case.method, case.path
                )
            });
        assert_eq!(target.local_path, case.local_path);
        assert!(!target.is_sse);
        assert!(target.session_id_hint.is_none());
    }
}

#[test]
fn resolve_proxy_target_rejects_invalid_percent_encoded_service_alias_segments() {
    for (method, path) in [
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
            "expected invalid percent-encoded service alias segment to be rejected for {method} {path}"
        );
    }
}

#[test]
fn resolve_proxy_target_rejects_wrong_methods_for_service_and_capability_alias_routes() {
    for (method, path) in [
        (
            "GET",
            "/v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run",
        ),
        ("POST", "/v1/agents/codexw-lab/sessions/sess_1/services"),
        ("POST", "/v1/agents/codexw-lab/sessions/sess_1/capabilities"),
    ] {
        assert!(
            resolve_proxy_target(method, path, "codexw-lab").is_none(),
            "expected service/capability alias route to reject wrong method for {method} {path}"
        );
    }
}
