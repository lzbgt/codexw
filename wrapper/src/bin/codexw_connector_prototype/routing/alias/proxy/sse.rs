use crate::routing::ProxyTarget;

pub(super) fn resolve_proxy_target(path: &str, agent_id: &str) -> Option<ProxyTarget> {
    let proxy_sse_prefix = format!("/v1/agents/{agent_id}/proxy_sse/");
    path.strip_prefix(&proxy_sse_prefix)
        .map(|stripped| super::passthrough_proxy_target(stripped, true))
}
