#[path = "routing/alias.rs"]
mod alias;
#[path = "routing/policy.rs"]
mod policy;
#[path = "routing/target.rs"]
mod target;

pub(crate) use target::ProxyTarget;

pub(super) fn resolve_proxy_target(
    method: &str,
    path: &str,
    agent_id: &str,
) -> Option<ProxyTarget> {
    alias::resolve_proxy_target(method, path, agent_id)
}

pub(super) fn is_allowed_local_proxy_target(method: &str, local_path: &str, is_sse: bool) -> bool {
    policy::is_allowed_local_proxy_target(method, local_path, is_sse)
}

pub(super) fn supports_client_lease_injection(method: &str, local_path: &str) -> bool {
    policy::supports_client_lease_injection(method, local_path)
}

#[cfg(test)]
pub(super) fn percent_decode_path_segment(value: &str) -> Option<String> {
    alias::percent_decode_path_segment(value)
}

#[cfg(test)]
pub(super) fn local_session_path(session_id: &str, suffix: &str) -> String {
    alias::local_session_path(session_id, suffix)
}

#[cfg(test)]
pub(super) fn decoded_session_ref_path(
    session_id: &str,
    category: &str,
    reference: &str,
) -> Option<String> {
    alias::decoded_session_ref_path(session_id, category, reference)
}

#[cfg(test)]
pub(super) fn decoded_session_ref_action_path(
    session_id: &str,
    category: &str,
    reference: &str,
    action: &str,
) -> Option<String> {
    alias::decoded_session_ref_action_path(session_id, category, reference, action)
}
