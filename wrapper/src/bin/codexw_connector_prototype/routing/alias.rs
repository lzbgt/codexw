#[path = "alias/proxy.rs"]
mod proxy;
#[path = "alias/session.rs"]
mod session;

use crate::routing::ProxyTarget;

pub(super) fn resolve_proxy_target(
    method: &str,
    path: &str,
    agent_id: &str,
) -> Option<ProxyTarget> {
    proxy::resolve_proxy_target(path, agent_id)
        .or_else(|| session::resolve_proxy_target(method, path, agent_id))
}
