use super::injection::is_supported_post_local_route;
use super::injection::local_path_segments;

pub(super) fn is_allowed_local_proxy_target(method: &str, local_path: &str, is_sse: bool) -> bool {
    let segments = local_path_segments(local_path);

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

fn is_supported_get_local_route(segments: &[&str]) -> bool {
    matches!(
        segments,
        ["healthz"]
            | ["api", "v1", "runtime"]
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
