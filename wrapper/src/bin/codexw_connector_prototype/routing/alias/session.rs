#[path = "session/lifecycle.rs"]
mod lifecycle;
#[path = "session/observation.rs"]
mod observation;
#[path = "session/workers.rs"]
mod workers;

use crate::routing::ProxyTarget;

pub(super) fn percent_decode_path_segment(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                if index + 2 >= bytes.len() {
                    return None;
                }
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok()?;
                let value = u8::from_str_radix(hex, 16).ok()?;
                decoded.push(value);
                index += 3;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

pub(super) fn local_session_path(session_id: &str, suffix: &str) -> String {
    format!("/api/v1/session/{session_id}/{suffix}")
}

pub(super) fn decoded_session_ref_path(
    session_id: &str,
    category: &str,
    reference: &str,
) -> Option<String> {
    let reference = percent_decode_path_segment(reference)?;
    Some(local_session_path(
        session_id,
        &format!("{category}/{reference}"),
    ))
}

pub(super) fn decoded_session_ref_action_path(
    session_id: &str,
    category: &str,
    reference: &str,
    action: &str,
) -> Option<String> {
    let reference = percent_decode_path_segment(reference)?;
    Some(local_session_path(
        session_id,
        &format!("{category}/{reference}/{action}"),
    ))
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
