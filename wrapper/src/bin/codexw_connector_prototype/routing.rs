#[derive(Debug, Clone)]
pub(super) struct ProxyTarget {
    pub(super) local_path: String,
    pub(super) is_sse: bool,
    pub(super) session_id_hint: Option<String>,
}

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
                [] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["attach"] => Some(ProxyTarget {
                    local_path: "/api/v1/session/attach".to_string(),
                    is_sse: false,
                    session_id_hint: Some(session_id),
                }),
                ["attachment", "renew"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/attachment/renew"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["attachment", "release"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/attachment/release"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["client-events"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/client_event"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["turns"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/turn/start"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["interrupt"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/turn/interrupt"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["transcript"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/transcript"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["shells"] => Some(ProxyTarget {
                    local_path: if method == "GET" {
                        format!("/api/v1/session/{session_id}/shells")
                    } else {
                        format!("/api/v1/session/{session_id}/shells/start")
                    },
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["shells", job_ref] if method == "GET" => {
                    let job_ref = percent_decode_path_segment(job_ref)?;
                    Some(ProxyTarget {
                        local_path: format!("/api/v1/session/{session_id}/shells/{job_ref}"),
                        is_sse: false,
                        session_id_hint: None,
                    })
                }
                ["shells", job_ref, "poll"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/shells/{job_ref}/poll"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["shells", job_ref, "send"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/shells/{job_ref}/send"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["shells", job_ref, "terminate"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/shells/{job_ref}/terminate"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/services"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref] if method == "GET" => {
                    let job_ref = percent_decode_path_segment(job_ref)?;
                    Some(ProxyTarget {
                        local_path: format!("/api/v1/session/{session_id}/services/{job_ref}"),
                        is_sse: false,
                        session_id_hint: None,
                    })
                }
                ["capabilities"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/capabilities"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["capabilities", capability] if method == "GET" => {
                    let capability = percent_decode_path_segment(capability)?;
                    Some(ProxyTarget {
                        local_path: format!(
                            "/api/v1/session/{session_id}/capabilities/{capability}"
                        ),
                        is_sse: false,
                        session_id_hint: None,
                    })
                }
                ["services", job_ref, "provide"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/services/{job_ref}/provide"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "depend"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/services/{job_ref}/depend"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "contract"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/services/{job_ref}/contract"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "relabel"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/services/{job_ref}/relabel"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "attach"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/services/{job_ref}/attach"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "wait"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/services/{job_ref}/wait"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["services", job_ref, "run"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/services/{job_ref}/run"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["events"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/events"),
                    is_sse: true,
                    session_id_hint: None,
                }),
                ["orchestration", "status"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/orchestration/status"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["orchestration", "workers"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/orchestration/workers"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                ["orchestration", "dependencies"] => Some(ProxyTarget {
                    local_path: format!("/api/v1/session/{session_id}/orchestration/dependencies"),
                    is_sse: false,
                    session_id_hint: None,
                }),
                _ => None,
            };
        }
    }

    let sessions_root = format!("/v1/agents/{agent_id}/sessions");
    if path == sessions_root || path == format!("{sessions_root}/") {
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

pub(super) fn is_allowed_local_proxy_target(method: &str, local_path: &str, is_sse: bool) -> bool {
    let trimmed = local_path.trim_matches('/');
    let segments: Vec<&str> = if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    };

    if is_sse {
        return method == "GET"
            && matches!(segments.as_slice(), ["api", "v1", "session", _, "events"]);
    }

    match method {
        "GET" => matches!(
            segments.as_slice(),
            ["healthz"]
                | ["api", "v1", "session"]
                | ["api", "v1", "session", _]
                | ["api", "v1", "session", _, "transcript"]
                | ["api", "v1", "session", _, "client_event"]
                | ["api", "v1", "session", _, "shells"]
                | ["api", "v1", "session", _, "shells", _]
                | ["api", "v1", "session", _, "services"]
                | ["api", "v1", "session", _, "services", _]
                | ["api", "v1", "session", _, "capabilities"]
                | ["api", "v1", "session", _, "capabilities", _]
                | ["api", "v1", "session", _, "orchestration", "status"]
                | ["api", "v1", "session", _, "orchestration", "dependencies"]
                | ["api", "v1", "session", _, "orchestration", "workers"]
        ),
        "POST" => matches!(
            segments.as_slice(),
            ["api", "v1", "session", "new"]
                | ["api", "v1", "session", "attach"]
                | ["api", "v1", "session", "client_event"]
                | ["api", "v1", "session", _, "attachment", "renew"]
                | ["api", "v1", "session", _, "attachment", "release"]
                | ["api", "v1", "session", _, "client_event"]
                | ["api", "v1", "session", _, "turn", "start"]
                | ["api", "v1", "session", _, "turn", "interrupt"]
                | ["api", "v1", "session", _, "shells", "start"]
                | ["api", "v1", "session", _, "shells", _, "poll"]
                | ["api", "v1", "session", _, "shells", _, "send"]
                | ["api", "v1", "session", _, "shells", _, "terminate"]
                | ["api", "v1", "session", _, "services", "update"]
                | ["api", "v1", "session", _, "services", _, "provide"]
                | ["api", "v1", "session", _, "services", _, "depend"]
                | ["api", "v1", "session", _, "services", _, "contract"]
                | ["api", "v1", "session", _, "services", _, "relabel"]
                | ["api", "v1", "session", _, "services", _, "attach"]
                | ["api", "v1", "session", _, "services", _, "wait"]
                | ["api", "v1", "session", _, "services", _, "run"]
        ),
        _ => false,
    }
}
