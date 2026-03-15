#[path = "routing/alias.rs"]
mod alias;
#[path = "routing/policy.rs"]
mod policy;

#[derive(Debug, Clone)]
pub(super) struct ProxyTarget {
    pub(super) local_path: String,
    pub(super) is_sse: bool,
    pub(super) session_id_hint: Option<String>,
}

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
