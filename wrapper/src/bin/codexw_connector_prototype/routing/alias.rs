#[path = "alias/proxy.rs"]
mod proxy;
#[path = "alias/session.rs"]
pub(super) mod session;

use crate::routing::ProxyTarget;

pub(super) fn resolve_proxy_target(
    method: &str,
    path: &str,
    agent_id: &str,
) -> Option<ProxyTarget> {
    proxy::resolve_proxy_target(path, agent_id)
        .or_else(|| session::resolve_proxy_target(method, path, agent_id))
}

#[cfg(test)]
pub(super) fn percent_decode_path_segment(value: &str) -> Option<String> {
    session::percent_decode_path_segment(value)
}

#[cfg(test)]
pub(super) fn local_session_path(session_id: &str, suffix: &str) -> String {
    session::local_session_path(session_id, suffix)
}

#[cfg(test)]
pub(super) fn decoded_session_ref_path(
    session_id: &str,
    category: &str,
    reference: &str,
) -> Option<String> {
    session::decoded_session_ref_path(session_id, category, reference)
}

#[cfg(test)]
pub(super) fn decoded_session_ref_action_path(
    session_id: &str,
    category: &str,
    reference: &str,
    action: &str,
) -> Option<String> {
    session::decoded_session_ref_action_path(session_id, category, reference, action)
}
