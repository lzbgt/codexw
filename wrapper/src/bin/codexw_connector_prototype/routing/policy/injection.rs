pub(super) fn supports_client_lease_injection(method: &str, local_path: &str) -> bool {
    method == "POST" && is_supported_post_local_route(local_path_segments(local_path).as_slice())
}

pub(super) fn local_path_segments(local_path: &str) -> Vec<&str> {
    let trimmed = local_path.trim_matches('/');
    if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    }
}

pub(super) fn is_supported_post_local_route(segments: &[&str]) -> bool {
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
