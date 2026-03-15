use crate::routing::ProxyTarget;

pub(super) fn resolve_proxy_target(path: &str, agent_id: &str) -> Option<ProxyTarget> {
    let proxy_prefix = format!("/v1/agents/{agent_id}/proxy/");
    if let Some(stripped) = path.strip_prefix(&proxy_prefix) {
        return Some(ProxyTarget {
            local_path: format!("/{}", stripped.trim_start_matches('/')),
            is_sse: false,
            session_id_hint: None,
        });
    }

    let proxy_sse_prefix = format!("/v1/agents/{agent_id}/proxy_sse/");
    if let Some(stripped) = path.strip_prefix(&proxy_sse_prefix) {
        return Some(ProxyTarget {
            local_path: format!("/{}", stripped.trim_start_matches('/')),
            is_sse: true,
            session_id_hint: None,
        });
    }

    None
}
