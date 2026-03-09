use std::sync::Arc;
use std::sync::RwLock;

use serde_json::Value;

use super::control::LocalApiCommand;
use super::control::new_command_queue;
use super::server::HttpRequest;
use super::server::route_request;
use super::snapshot::LocalApiSnapshot;

fn sample_snapshot() -> Arc<RwLock<LocalApiSnapshot>> {
    Arc::new(RwLock::new(LocalApiSnapshot {
        session_id: "sess_test".to_string(),
        cwd: "/tmp/repo".to_string(),
        thread_id: Some("thread_123".to_string()),
        active_turn_id: Some("turn_456".to_string()),
        objective: Some("continue".to_string()),
        turn_running: true,
        started_turn_count: 3,
        completed_turn_count: 2,
        active_personality: Some("balanced".to_string()),
    }))
}

fn get_request(path: &str) -> HttpRequest {
    HttpRequest {
        method: "GET".to_string(),
        path: path.to_string(),
        headers: Default::default(),
        body: Vec::new(),
    }
}

fn post_json_request(path: &str, body: Value) -> HttpRequest {
    HttpRequest {
        method: "POST".to_string(),
        path: path.to_string(),
        headers: Default::default(),
        body: serde_json::to_vec(&body).expect("serialize body"),
    }
}

fn json_body(response_body: &[u8]) -> Value {
    serde_json::from_slice(response_body).expect("response body should be valid json")
}

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
    let response = route_request(&request, &sample_snapshot(), &new_command_queue(), Some("secret"));
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["session_id"], "sess_test");
    assert_eq!(body["thread_id"], "thread_123");
    assert_eq!(body["working"], Value::Bool(true));
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
    assert_eq!(json_body(&response.body)["error"]["code"], "session_not_found");
}

#[test]
fn turn_start_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/turn/start",
            serde_json::json!({
                "session_id": "sess_test",
                "input": { "text": "review this diff" }
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    assert_eq!(json_body(&response.body)["accepted"], Value::Bool(true));
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::StartTurn {
            session_id: "sess_test".to_string(),
            prompt: "review this diff".to_string(),
        })
    );
}

#[test]
fn turn_start_requires_attached_thread() {
    let snapshot = Arc::new(RwLock::new(LocalApiSnapshot {
        thread_id: None,
        ..sample_snapshot().read().expect("snapshot").clone()
    }));
    let response = route_request(
        &post_json_request(
            "/api/v1/turn/start",
            serde_json::json!({
                "session_id": "sess_test",
                "input": { "text": "review this diff" }
            }),
        ),
        &snapshot,
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 409);
    assert_eq!(json_body(&response.body)["error"]["code"], "thread_not_attached");
}

#[test]
fn turn_interrupt_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/turn/interrupt",
            serde_json::json!({ "session_id": "sess_test" }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::InterruptTurn {
            session_id: "sess_test".to_string(),
        })
    );
}
