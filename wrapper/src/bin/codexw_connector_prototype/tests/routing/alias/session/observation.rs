use crate::routing::resolve_proxy_target;

#[test]
fn resolve_proxy_target_maps_observation_alias_routes() {
    for (path, local_path, is_sse) in [
        (
            "/v1/agents/codexw-lab/sessions/sess_1/events",
            "/api/v1/session/sess_1/events",
            true,
        ),
        (
            "/v1/agents/codexw-lab/sessions/sess_1/orchestration/status",
            "/api/v1/session/sess_1/orchestration/status",
            false,
        ),
        (
            "/v1/agents/codexw-lab/sessions/sess_1/orchestration/workers",
            "/api/v1/session/sess_1/orchestration/workers",
            false,
        ),
        (
            "/v1/agents/codexw-lab/sessions/sess_1/orchestration/dependencies",
            "/api/v1/session/sess_1/orchestration/dependencies",
            false,
        ),
    ] {
        let target = resolve_proxy_target("GET", path, "codexw-lab")
            .unwrap_or_else(|| panic!("expected observation alias route mapping for GET {path}"));
        assert_eq!(target.local_path, local_path);
        assert_eq!(target.is_sse, is_sse);
        assert!(target.session_id_hint.is_none());
    }
}

#[test]
fn resolve_proxy_target_rejects_wrong_methods_for_observation_alias_routes() {
    for (method, path) in [
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
            "expected observation alias route to reject wrong method for {method} {path}"
        );
    }
}
