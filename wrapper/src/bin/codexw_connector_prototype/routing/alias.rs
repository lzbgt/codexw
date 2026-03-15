use crate::routing::ProxyTarget;

fn percent_decode_path_segment(value: &str) -> Option<String> {
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

fn local_session_path(session_id: &str, suffix: &str) -> String {
    format!("/api/v1/session/{session_id}/{suffix}")
}

fn decoded_session_ref_path(session_id: &str, category: &str, reference: &str) -> Option<String> {
    let reference = percent_decode_path_segment(reference)?;
    Some(local_session_path(
        session_id,
        &format!("{category}/{reference}"),
    ))
}

fn decoded_session_ref_action_path(
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
    let proxy_prefix = format!("/v1/agents/{agent_id}/proxy/");
    if let Some(stripped) = path.strip_prefix(&proxy_prefix) {
        return Some(ProxyTarget {
            local_path: format!("/{}", stripped.trim_start_matches('/')),
            is_sse: false,
            session_id_hint: None,
        });
    }

    let proxy_sse_prefix = format!("/v1/agents/{agent_id}/proxy_sse/");
    if let Some(stripped) = path.strip_prefix(&proxy_sse_prefix) {
        return Some(ProxyTarget {
            local_path: format!("/{}", stripped.trim_start_matches('/')),
            is_sse: true,
            session_id_hint: None,
        });
    }

    let session_prefix = format!("/v1/agents/{agent_id}/sessions/");
    if let Some(stripped) = path.strip_prefix(&session_prefix) {
        let segments: Vec<&str> = stripped
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        if let Some((session_id, rest)) = segments.split_first() {
            let session_id = (*session_id).to_string();
            return match rest {
                [] if method == "GET" => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["attach"] if method == "POST" => Some(ProxyTarget {
                    local_path: "/api/v1/session/attach".to_string(),
                    is_sse: false,
                    session_id_hint: Some(session_id),
                }),
                ["attachment", "renew"] if method == "POST" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "attachment/renew"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["attachment", "release"] if method == "POST" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "attachment/release"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["client-events"] if method == "POST" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "client_event"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["turns"] if method == "POST" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "turn/start"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["interrupt"] if method == "POST" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "turn/interrupt"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["transcript"] if method == "GET" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "transcript"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["shells"] if method == "GET" || method == "POST" => Some(ProxyTarget {
                    local_path: if method == "GET" {
                        local_session_path(&session_id, "shells")
                    } else {
                        local_session_path(&session_id, "shells/start")
                    },
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["shells", job_ref] if method == "GET" => Some(ProxyTarget {
                    local_path: decoded_session_ref_path(&session_id, "shells", job_ref)?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["shells", job_ref, "poll"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "shells",
                        job_ref,
                        "poll",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["shells", job_ref, "send"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "shells",
                        job_ref,
                        "send",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["shells", job_ref, "terminate"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "shells",
                        job_ref,
                        "terminate",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services"] if method == "GET" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "services"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref] if method == "GET" => Some(ProxyTarget {
                    local_path: decoded_session_ref_path(&session_id, "services", job_ref)?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["capabilities"] if method == "GET" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "capabilities"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["capabilities", capability] if method == "GET" => Some(ProxyTarget {
                    local_path: decoded_session_ref_path(&session_id, "capabilities", capability)?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "provide"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "services",
                        job_ref,
                        "provide",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "depend"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "services",
                        job_ref,
                        "depend",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "contract"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "services",
                        job_ref,
                        "contract",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "relabel"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "services",
                        job_ref,
                        "relabel",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "attach"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "services",
                        job_ref,
                        "attach",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "wait"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "services",
                        job_ref,
                        "wait",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "run"] if method == "POST" => Some(ProxyTarget {
                    local_path: decoded_session_ref_action_path(
                        &session_id,
                        "services",
                        job_ref,
                        "run",
                    )?,
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["events"] => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "events"),
                    is_sse: true,
                    session_id_hint: None,
                }),
                ["orchestration", "status"] if method == "GET" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "orchestration/status"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["orchestration", "workers"] if method == "GET" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "orchestration/workers"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["orchestration", "dependencies"] if method == "GET" => Some(ProxyTarget {
                    local_path: local_session_path(&session_id, "orchestration/dependencies"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                _ => None,
            };
        }
    }

    let sessions_root = format!("/v1/agents/{agent_id}/sessions");
    if (path == sessions_root || path == format!("{sessions_root}/"))
        && (method == "GET" || method == "POST")
    {
        return Some(ProxyTarget {
            local_path: if method == "POST" {
                "/api/v1/session/new".to_string()
            } else {
                "/api/v1/session".to_string()
            },
            is_sse: false,
            session_id_hint: None,
        });
    }

    None
}
