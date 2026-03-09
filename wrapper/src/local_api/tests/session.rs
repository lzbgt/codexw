use serde_json::Value;

use super::LocalApiCommand;
use super::get_request;
use super::json_body;
use super::new_command_queue;
use super::post_json_request;
use super::route_request;
use super::sample_snapshot;

#[test]
fn healthz_is_public() {
    let response = route_request(
        &get_request("/healthz"),
        &sample_snapshot(),
        &new_command_queue(),
        Some("secret"),
    );
    assert_eq!(response.status, 200);
    assert_eq!(json_body(&response.body)["ok"], Value::Bool(true));
}

#[test]
fn session_requires_auth_when_token_is_configured() {
    let response = route_request(
        &get_request("/api/v1/session"),
        &sample_snapshot(),
        &new_command_queue(),
        Some("secret"),
    );
    assert_eq!(response.status, 401);
    assert_eq!(json_body(&response.body)["error"]["code"], "unauthorized");
}

#[test]
fn session_snapshot_is_returned_with_valid_token() {
    let mut request = get_request("/api/v1/session");
    request
        .headers
        .insert("authorization".to_string(), "Bearer secret".to_string());
    let response = route_request(
        &request,
        &sample_snapshot(),
        &new_command_queue(),
        Some("secret"),
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["session_id"], "sess_test");
    assert_eq!(body["session"]["id"], "sess_test");
    assert_eq!(body["session"]["scope"], "process");
    assert_eq!(body["session"]["attachment"]["id"], "attach:sess_test");
    assert_eq!(body["session"]["attachment"]["client_id"], "client_web");
    assert_eq!(body["session"]["attachment"]["lease_seconds"], 300);
    assert_eq!(body["session"]["attached_thread_id"], "thread_123");
    assert_eq!(body["thread_id"], "thread_123");
    assert_eq!(body["working"], Value::Bool(true));
    assert_eq!(body["orchestration"]["main_agent_state"], "blocked");
}

#[test]
fn session_id_route_reuses_same_snapshot_payload() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["session_id"], "sess_test");
    assert_eq!(body["session"]["active_turn_id"], "turn_456");
    assert_eq!(body["session"]["attachment"]["scope"], "process");
    assert_eq!(body["session"]["attachment"]["client_id"], "client_web");
    assert_eq!(body["active_turn_id"], "turn_456");
}

#[test]
fn unknown_session_id_returns_not_found() {
    let response = route_request(
        &get_request("/api/v1/session/sess_other"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 404);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "session_not_found"
    );
}

#[test]
fn session_new_enqueues_fresh_thread_start() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/new",
            serde_json::json!({
                "client_id": "client_web",
                "lease_seconds": 120
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["session_id"], "sess_test");
    assert_eq!(body["session"]["scope"], "process");
    assert_eq!(body["attachment"]["id"], "attach:sess_test");
    assert_eq!(body["process_scoped"], Value::Bool(true));
    assert_eq!(body["operation"]["kind"], "session.new");
    assert_eq!(body["requested_action"], "start_thread");
    assert_eq!(body["operation"]["requested_client_id"], "client_web");
    assert_eq!(body["operation"]["requested_lease_seconds"], 120);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::StartSessionThread {
            session_id: "sess_test".to_string(),
            client_id: Some("client_web".to_string()),
            lease_seconds: Some(120),
        })
    );
}

#[test]
fn session_attach_enqueues_thread_resume() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/attach",
            serde_json::json!({
                "session_id": "sess_test",
                "thread_id": "thread_resume_target",
                "client_id": "client_web",
                "lease_seconds": 180
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["session"]["id"], "sess_test");
    assert_eq!(body["session"]["scope"], "process");
    assert_eq!(body["attachment"]["attached_thread_id"], "thread_123");
    assert_eq!(body["operation"]["kind"], "session.attach");
    assert_eq!(
        body["operation"]["target_thread_id"],
        "thread_resume_target"
    );
    assert_eq!(body["operation"]["requested_client_id"], "client_web");
    assert_eq!(body["operation"]["requested_lease_seconds"], 180);
    assert_eq!(body["target_thread_id"], "thread_resume_target");
    assert_eq!(body["requested_action"], "attach_thread");
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::AttachSessionThread {
            session_id: "sess_test".to_string(),
            thread_id: "thread_resume_target".to_string(),
            client_id: Some("client_web".to_string()),
            lease_seconds: Some(180),
        })
    );
}

#[test]
fn session_attach_rejects_unknown_session() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/attach",
            serde_json::json!({
                "session_id": "sess_other",
                "thread_id": "thread_resume_target"
            }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 404);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "session_not_found"
    );
}

#[test]
fn session_attach_requires_thread_id() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/attach",
            serde_json::json!({ "session_id": "sess_test" }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 400);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "validation_error"
    );
}

#[test]
fn session_new_rejects_conflicting_active_client() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/new",
            serde_json::json!({
                "client_id": "client_mobile",
                "lease_seconds": 30
            }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 409);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "attachment_conflict"
    );
    let body = json_body(&response.body);
    assert_eq!(body["error"]["status"], 409);
    assert_eq!(body["error"]["retryable"], false);
    assert_eq!(
        body["error"]["details"]["requested_client_id"],
        "client_mobile"
    );
    assert_eq!(
        body["error"]["details"]["current_attachment"]["client_id"],
        "client_web"
    );
    assert_eq!(
        body["error"]["details"]["current_attachment"]["lease_active"],
        true
    );
}

#[test]
fn session_new_rejects_non_string_client_id_with_field_details() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/new",
            serde_json::json!({
                "client_id": 123,
                "lease_seconds": 30
            }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 400);
    let body = json_body(&response.body);
    assert_eq!(body["error"]["code"], "validation_error");
    assert_eq!(body["error"]["status"], 400);
    assert_eq!(body["error"]["retryable"], false);
    assert_eq!(body["error"]["details"]["field"], "client_id");
    assert_eq!(body["error"]["details"]["expected"], "string");
}

#[test]
fn session_new_rejects_anonymous_request_when_lease_is_active() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/new",
            serde_json::json!({
                "lease_seconds": 30
            }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 409);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "attachment_conflict"
    );
}

#[test]
fn session_attachment_renew_enqueues_lease_update() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/attachment/renew",
            serde_json::json!({
                "client_id": "client_web",
                "lease_seconds": 60
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["operation"]["kind"], "attachment.renew");
    assert_eq!(body["operation"]["requested_client_id"], "client_web");
    assert_eq!(body["operation"]["requested_lease_seconds"], 60);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::RenewAttachmentLease {
            session_id: "sess_test".to_string(),
            client_id: Some("client_web".to_string()),
            lease_seconds: 60,
        })
    );
}

#[test]
fn session_attachment_release_enqueues_release() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/attachment/release",
            serde_json::json!({
                "client_id": "client_web"
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["operation"]["kind"], "attachment.release");
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::ReleaseAttachment {
            session_id: "sess_test".to_string(),
            client_id: Some("client_web".to_string()),
        })
    );
}

#[test]
fn session_attachment_release_rejects_wrong_client() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/attachment/release",
            serde_json::json!({
                "client_id": "client_mobile"
            }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 409);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "attachment_conflict"
    );
    let body = json_body(&response.body);
    assert_eq!(
        body["error"]["details"]["requested_client_id"],
        "client_mobile"
    );
    assert_eq!(
        body["error"]["details"]["current_attachment"]["client_id"],
        "client_web"
    );
}

#[test]
fn top_level_client_event_requires_known_session_id() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/client_event",
            serde_json::json!({
                "session_id": "sess_other",
                "event": "selection.changed"
            }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 404);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "session_not_found"
    );
}

#[test]
fn client_event_rejects_conflicting_active_client() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/client_event",
            serde_json::json!({
                "client_id": "client_mobile",
                "event": "selection.changed"
            }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 409);
    let body = json_body(&response.body);
    assert_eq!(body["error"]["code"], "attachment_conflict");
    assert_eq!(
        body["error"]["details"]["requested_client_id"],
        "client_mobile"
    );
    assert_eq!(
        body["error"]["details"]["current_attachment"]["client_id"],
        "client_web"
    );
}
