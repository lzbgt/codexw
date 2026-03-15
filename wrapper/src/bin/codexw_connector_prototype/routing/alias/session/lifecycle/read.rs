use crate::routing::ProxyTarget;

use super::super::local_session_path;

pub(super) fn resolve_proxy_target(
    method: &str,
    session_id: &str,
    rest: &[&str],
) -> Option<ProxyTarget> {
    match rest {
        [] if method == "GET" => Some(ProxyTarget {
            local_path: format!("/api/v1/session/{session_id}"),
            is_sse: false,
            session_id_hint: None,
        }),
        ["transcript"] if method == "GET" => Some(ProxyTarget {
            local_path: local_session_path(session_id, "transcript"),
            is_sse: false,
            session_id_hint: None,
        }),
        _ => None,
    }
}

pub(super) fn resolve_sessions_root_proxy_target(
    method: &str,
    path: &str,
    agent_id: &str,
) -> Option<ProxyTarget> {
    let sessions_root = format!("/v1/agents/{agent_id}/sessions");
    if path == sessions_root || path == format!("{sessions_root}/") {
        return (method == "GET").then_some(ProxyTarget {
            local_path: "/api/v1/session".to_string(),
            is_sse: false,
            session_id_hint: None,
        });
    }

    None
}
