use crate::routing::ProxyTarget;

use super::{decoded_session_ref_action_path, decoded_session_ref_path, local_session_path};

pub(super) fn resolve_proxy_target(
    method: &str,
    session_id: &str,
    rest: &[&str],
) -> Option<ProxyTarget> {
    match rest {
        ["shells"] if method == "GET" || method == "POST" => Some(ProxyTarget {
            local_path: if method == "GET" {
                local_session_path(session_id, "shells")
            } else {
                local_session_path(session_id, "shells/start")
            },
            is_sse: false,
            session_id_hint: None,
        }),
        ["shells", job_ref] if method == "GET" => Some(ProxyTarget {
            local_path: decoded_session_ref_path(session_id, "shells", job_ref)?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["shells", job_ref, "poll"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(session_id, "shells", job_ref, "poll")?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["shells", job_ref, "send"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(session_id, "shells", job_ref, "send")?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["shells", job_ref, "terminate"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(
                session_id,
                "shells",
                job_ref,
                "terminate",
            )?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["services"] if method == "GET" => Some(ProxyTarget {
            local_path: local_session_path(session_id, "services"),
            is_sse: false,
            session_id_hint: None,
        }),
        ["services", job_ref] if method == "GET" => Some(ProxyTarget {
            local_path: decoded_session_ref_path(session_id, "services", job_ref)?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["capabilities"] if method == "GET" => Some(ProxyTarget {
            local_path: local_session_path(session_id, "capabilities"),
            is_sse: false,
            session_id_hint: None,
        }),
        ["capabilities", capability] if method == "GET" => Some(ProxyTarget {
            local_path: decoded_session_ref_path(session_id, "capabilities", capability)?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["services", job_ref, "provide"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(
                session_id, "services", job_ref, "provide",
            )?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["services", job_ref, "depend"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(session_id, "services", job_ref, "depend")?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["services", job_ref, "contract"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(
                session_id, "services", job_ref, "contract",
            )?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["services", job_ref, "relabel"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(
                session_id, "services", job_ref, "relabel",
            )?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["services", job_ref, "attach"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(session_id, "services", job_ref, "attach")?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["services", job_ref, "wait"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(session_id, "services", job_ref, "wait")?,
            is_sse: false,
            session_id_hint: None,
        }),
        ["services", job_ref, "run"] if method == "POST" => Some(ProxyTarget {
            local_path: decoded_session_ref_action_path(session_id, "services", job_ref, "run")?,
            is_sse: false,
            session_id_hint: None,
        }),
        _ => None,
    }
}
