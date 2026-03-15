#[path = "session/lifecycle.rs"]
mod lifecycle;
#[path = "session/observation.rs"]
mod observation;
#[path = "session/path.rs"]
mod path;
#[path = "session/workers.rs"]
mod workers;

use crate::routing::ProxyTarget;

#[cfg(test)]
pub(super) fn percent_decode_path_segment(value: &str) -> Option<String> {
    path::percent_decode_path_segment(value)
}

pub(super) fn local_session_path(session_id: &str, suffix: &str) -> String {
    path::local_session_path(session_id, suffix)
}

pub(super) fn decoded_session_ref_path(
    session_id: &str,
    category: &str,
    reference: &str,
) -> Option<String> {
    path::decoded_session_ref_path(session_id, category, reference)
}

pub(super) fn decoded_session_ref_action_path(
    session_id: &str,
    category: &str,
    reference: &str,
    action: &str,
) -> Option<String> {
    path::decoded_session_ref_action_path(session_id, category, reference, action)
}

pub(super) fn resolve_proxy_target(
    method: &str,
    path: &str,
    agent_id: &str,
) -> Option<ProxyTarget> {
    let session_prefix = format!("/v1/agents/{agent_id}/sessions/");
    if let Some(stripped) = path.strip_prefix(&session_prefix) {
        let segments: Vec<&str> = stripped
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        if let Some((session_id, rest)) = segments.split_first() {
            let session_id = (*session_id).to_string();
            return lifecycle::resolve_proxy_target(method, &session_id, rest)
                .or_else(|| workers::resolve_proxy_target(method, &session_id, rest))
                .or_else(|| observation::resolve_proxy_target(method, &session_id, rest));
        }
    }

    lifecycle::resolve_sessions_root_proxy_target(method, path, agent_id)
}
