use crate::routing::resolve_proxy_target;

#[test]
fn resolve_proxy_target_maps_http_proxy_routes() {
    let create = resolve_proxy_target(
        "POST",
        "/v1/agents/codexw-lab/proxy/api/v1/session/new",
        "codexw-lab",
    )
    .expect("create route");
    assert_eq!(create.local_path, "/api/v1/session/new");
    assert!(!create.is_sse);
    assert!(create.session_id_hint.is_none());

    let inspect = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/proxy/api/v1/session/sess_1",
        "codexw-lab",
    )
    .expect("inspect route");
    assert_eq!(inspect.local_path, "/api/v1/session/sess_1");
    assert!(!inspect.is_sse);
    assert!(inspect.session_id_hint.is_none());
}

#[test]
fn resolve_proxy_target_rejects_wrong_agent_for_http_proxy_routes() {
    assert!(
        resolve_proxy_target(
            "POST",
            "/v1/agents/other/proxy/api/v1/session/new",
            "codexw-lab",
        )
        .is_none()
    );
}
