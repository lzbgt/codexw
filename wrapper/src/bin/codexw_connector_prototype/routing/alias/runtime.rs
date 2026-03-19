use crate::routing::ProxyTarget;

pub(super) fn resolve_proxy_target(
    method: &str,
    path: &str,
    agent_id: &str,
) -> Option<ProxyTarget> {
    if method != "GET" {
        return None;
    }

    let runtime_path = format!("/v1/agents/{agent_id}/runtime");
    if path != runtime_path {
        return None;
    }

    Some(ProxyTarget {
        local_path: "/api/v1/runtime".to_string(),
        is_sse: false,
        session_id_hint: None,
    })
}
