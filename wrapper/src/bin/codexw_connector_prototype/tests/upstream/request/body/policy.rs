use std::collections::HashMap;

use crate::upstream::prepare_upstream_body;

use super::request_with;
use super::target_with;

#[test]
fn prepare_upstream_body_passthroughs_non_post_requests() {
    let request = request_with(
        "GET",
        "/v1/agents/codexw-lab/proxy/api/v1/session/sess_1/transcript",
        HashMap::from([
            ("content-type".to_string(), "text/plain".to_string()),
            ("x-codexw-client-id".to_string(), "mobile-ios".to_string()),
        ]),
        b"keep-me".to_vec(),
    );

    let (content_type, body) = prepare_upstream_body(
        &request,
        &target_with("/api/v1/session/sess_1/transcript", None),
    )
    .expect("passthrough body");

    assert_eq!(content_type.as_deref(), Some("text/plain"));
    assert_eq!(body, b"keep-me");
}

#[test]
fn prepare_upstream_body_passthroughs_post_routes_without_injection_policy() {
    let request = request_with(
        "POST",
        "/v1/agents/codexw-lab/proxy/api/v1/session/sess_1/internal/debug",
        HashMap::from([
            (
                "content-type".to_string(),
                "application/octet-stream".to_string(),
            ),
            ("x-codexw-client-id".to_string(), "mobile-ios".to_string()),
        ]),
        b"opaque-body".to_vec(),
    );

    let (content_type, body) = prepare_upstream_body(
        &request,
        &target_with("/api/v1/session/sess_1/internal/debug", None),
    )
    .expect("passthrough body");

    assert_eq!(content_type.as_deref(), Some("application/octet-stream"));
    assert_eq!(body, b"opaque-body");
}
