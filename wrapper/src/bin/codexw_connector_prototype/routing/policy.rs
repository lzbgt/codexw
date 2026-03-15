#[path = "policy/allowlist.rs"]
mod allowlist;
#[path = "policy/injection.rs"]
mod injection;

pub(super) fn is_allowed_local_proxy_target(method: &str, local_path: &str, is_sse: bool) -> bool {
    allowlist::is_allowed_local_proxy_target(method, local_path, is_sse)
}

pub(super) fn supports_client_lease_injection(method: &str, local_path: &str) -> bool {
    injection::supports_client_lease_injection(method, local_path)
}
