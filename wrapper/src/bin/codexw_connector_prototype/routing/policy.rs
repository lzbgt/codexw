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
        "GET" => is_supported_get_local_route(segments.as_slice()),
        "POST" => is_supported_post_local_route(segments.as_slice()),
        _ => false,
    }
}

pub(super) fn supports_client_lease_injection(method: &str, local_path: &str) -> bool {
    if method != "POST" {
        return false;
    }
    let trimmed = local_path.trim_matches('/');
    let segments: Vec<&str> = if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    };
    is_supported_post_local_route(segments.as_slice())
}

fn is_supported_get_local_route(segments: &[&str]) -> bool {
    matches!(
        segments,
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
    )
}

fn is_supported_post_local_route(segments: &[&str]) -> bool {
    matches!(
        segments,
        ["api", "v1", "session", "new"]
            | ["api", "v1", "session", "attach"]
            | ["api", "v1", "session", "client_event"]
            | ["api", "v1", "turn", "start"]
            | ["api", "v1", "turn", "interrupt"]
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
    )
}
