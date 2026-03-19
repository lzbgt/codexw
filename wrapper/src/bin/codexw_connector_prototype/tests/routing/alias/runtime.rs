use crate::routing::resolve_proxy_target;

#[test]
fn resolve_proxy_target_maps_runtime_alias_route() {
    let target = resolve_proxy_target("GET", "/v1/agents/codexw-lab/runtime", "codexw-lab")
        .expect("runtime alias route should resolve");
    assert_eq!(target.local_path, "/api/v1/runtime");
    assert!(!target.is_sse);
    assert!(target.session_id_hint.is_none());
}

#[test]
fn resolve_proxy_target_rejects_wrong_method_for_runtime_alias_route() {
    assert!(resolve_proxy_target("POST", "/v1/agents/codexw-lab/runtime", "codexw-lab").is_none());
}

#[test]
fn resolve_proxy_target_rejects_wrong_agent_for_runtime_alias_route() {
    assert!(resolve_proxy_target("GET", "/v1/agents/other/runtime", "codexw-lab").is_none());
}
