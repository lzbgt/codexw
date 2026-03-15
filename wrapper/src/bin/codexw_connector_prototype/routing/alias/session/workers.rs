#[path = "workers/services.rs"]
mod services;
#[path = "workers/shells.rs"]
mod shells;

use crate::routing::ProxyTarget;

pub(super) fn resolve_proxy_target(
    method: &str,
    session_id: &str,
    rest: &[&str],
) -> Option<ProxyTarget> {
    shells::resolve_proxy_target(method, session_id, rest)
        .or_else(|| services::resolve_proxy_target(method, session_id, rest))
}
