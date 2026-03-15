#[path = "proxy/http.rs"]
mod http;
#[path = "proxy/sse.rs"]
mod sse;

use crate::routing::ProxyTarget;

pub(super) fn resolve_proxy_target(path: &str, agent_id: &str) -> Option<ProxyTarget> {
    http::resolve_proxy_target(path, agent_id).or_else(|| sse::resolve_proxy_target(path, agent_id))
}

fn passthrough_proxy_target(stripped: &str, is_sse: bool) -> ProxyTarget {
    ProxyTarget {
        local_path: format!("/{}", stripped.trim_start_matches('/')),
        is_sse,
        session_id_hint: None,
    }
}
