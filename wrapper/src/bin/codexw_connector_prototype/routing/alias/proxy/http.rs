use crate::routing::ProxyTarget;

pub(super) fn resolve_proxy_target(path: &str, agent_id: &str) -> Option<ProxyTarget> {
    let proxy_prefix = format!("/v1/agents/{agent_id}/proxy/");
    path.strip_prefix(&proxy_prefix)
        .map(|stripped| super::passthrough_proxy_target(stripped, false))
}
