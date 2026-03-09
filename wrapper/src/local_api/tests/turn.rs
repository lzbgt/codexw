use std::sync::Arc;
use std::sync::RwLock;

use super::LocalApiCommand;
use super::json_body;
use super::new_command_queue;
use super::post_json_request;
use super::route_request;
use super::sample_snapshot;
use crate::local_api::snapshot::LocalApiSnapshot;

#[test]
fn turn_start_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/turn/start",
            serde_json::json!({
                "session_id": "sess_test",
                "client_id": "client_web",
                "input": { "text": "review this diff" }
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        json_body(&response.body)["accepted"],
        serde_json::Value::Bool(true)
    );
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
fn session_scoped_turn_start_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/turn/start",
            serde_json::json!({
                "client_id": "client_web",
                "input": { "text": "review this diff" }
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["session"]["id"], "sess_test");
    assert_eq!(body["operation"]["kind"], "turn.start");
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
                "client_id": "client_web",
                "input": { "text": "review this diff" }
            }),
        ),
        &snapshot,
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 409);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "thread_not_attached"
    );
}

#[test]
fn turn_interrupt_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/turn/interrupt",
            serde_json::json!({ "session_id": "sess_test", "client_id": "client_web" }),
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

#[test]
fn session_scoped_turn_interrupt_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/turn/interrupt",
            serde_json::json!({ "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["session"]["id"], "sess_test");
    assert_eq!(body["operation"]["kind"], "turn.interrupt");
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::InterruptTurn {
            session_id: "sess_test".to_string(),
        })
    );
}

#[test]
fn session_scoped_turn_start_rejects_conflicting_attachment_client() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/turn/start",
            serde_json::json!({
                "client_id": "client_mobile",
                "input": { "text": "review this diff" }
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
fn session_scoped_turn_interrupt_rejects_anonymous_request_when_lease_active() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/turn/interrupt",
            serde_json::json!({}),
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
