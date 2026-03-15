#[path = "body/json.rs"]
mod json;
#[path = "body/policy.rs"]
mod policy;

use std::collections::HashMap;

use crate::http::HttpRequest;
use crate::routing::ProxyTarget;

pub(super) fn request_with(
    method: &str,
    path: &str,
    headers: HashMap<String, String>,
    body: Vec<u8>,
) -> HttpRequest {
    HttpRequest {
        method: method.to_string(),
        path: path.to_string(),
        headers,
        body,
    }
}

pub(super) fn target_with(local_path: &str, session_id_hint: Option<&str>) -> ProxyTarget {
    ProxyTarget {
        local_path: local_path.to_string(),
        is_sse: false,
        session_id_hint: session_id_hint.map(str::to_string),
    }
}
