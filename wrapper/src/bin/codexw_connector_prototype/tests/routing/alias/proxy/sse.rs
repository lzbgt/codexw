use crate::routing::resolve_proxy_target;

#[test]
fn resolve_proxy_target_maps_sse_proxy_routes() {
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
fn resolve_proxy_target_rejects_wrong_agent_for_sse_proxy_routes() {
    assert!(
        resolve_proxy_target(
            "GET",
            "/v1/agents/other/proxy_sse/api/v1/session/sess_1/events",
            "codexw-lab",
        )
        .is_none()
    );
}
