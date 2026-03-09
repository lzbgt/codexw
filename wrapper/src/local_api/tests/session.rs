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
        &post_json_request("/api/v1/session/new", serde_json::json!({})),
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
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::StartSessionThread {
            session_id: "sess_test".to_string(),
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
                "thread_id": "thread_resume_target"
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
    assert_eq!(body["target_thread_id"], "thread_resume_target");
    assert_eq!(body["requested_action"], "attach_thread");
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::AttachSessionThread {
            session_id: "sess_test".to_string(),
            thread_id: "thread_resume_target".to_string(),
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
