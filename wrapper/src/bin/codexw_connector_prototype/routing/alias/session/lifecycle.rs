#[path = "lifecycle/mutate.rs"]
mod mutate;
#[path = "lifecycle/read.rs"]
mod read;

use crate::routing::ProxyTarget;

pub(super) fn resolve_proxy_target(
    method: &str,
    session_id: &str,
    rest: &[&str],
) -> Option<ProxyTarget> {
    read::resolve_proxy_target(method, session_id, rest)
        .or_else(|| mutate::resolve_proxy_target(method, session_id, rest))
}

pub(super) fn resolve_sessions_root_proxy_target(
    method: &str,
    path: &str,
    agent_id: &str,
) -> Option<ProxyTarget> {
    read::resolve_sessions_root_proxy_target(method, path, agent_id)
        .or_else(|| mutate::resolve_sessions_root_proxy_target(method, path, agent_id))
}
