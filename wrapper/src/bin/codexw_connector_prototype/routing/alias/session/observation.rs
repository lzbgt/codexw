use crate::routing::ProxyTarget;

use super::local_session_path;

pub(super) fn resolve_proxy_target(
    method: &str,
    session_id: &str,
    rest: &[&str],
) -> Option<ProxyTarget> {
    match rest {
        ["events"] => Some(ProxyTarget {
            local_path: local_session_path(session_id, "events"),
            is_sse: true,
            session_id_hint: None,
        }),
        ["orchestration", "status"] if method == "GET" => Some(ProxyTarget {
            local_path: local_session_path(session_id, "orchestration/status"),
            is_sse: false,
            session_id_hint: None,
        }),
        ["orchestration", "workers"] if method == "GET" => Some(ProxyTarget {
            local_path: local_session_path(session_id, "orchestration/workers"),
            is_sse: false,
            session_id_hint: None,
        }),
        ["orchestration", "dependencies"] if method == "GET" => Some(ProxyTarget {
            local_path: local_session_path(session_id, "orchestration/dependencies"),
            is_sse: false,
            session_id_hint: None,
        }),
        _ => None,
    }
}
