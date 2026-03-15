use crate::routing::resolve_proxy_target;

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

    let inspect = resolve_proxy_target(
        "GET",
        "/v1/agents/codexw-lab/proxy/api/v1/session/sess_1",
        "codexw-lab",
    )
    .expect("inspect route");
    assert_eq!(inspect.local_path, "/api/v1/session/sess_1");
    assert!(!inspect.is_sse);
    assert!(inspect.session_id_hint.is_none());

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
