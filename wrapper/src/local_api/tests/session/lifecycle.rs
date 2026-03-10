use serde_json::Value;

use super::super::LocalApiCommand;
use super::super::json_body;
use super::super::new_command_queue;
use super::super::post_json_request;
use super::super::route_request;
use super::super::sample_snapshot;
use super::assert_json_path_eq;

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
fn session_lifecycle_and_inspection_routes_have_explicit_contract_coverage() {
    let post_cases = [
        (
            "/api/v1/session/new",
            serde_json::json!({
                "client_id": "client_web",
                "lease_seconds": 120
            }),
            "session.new",
        ),
        (
            "/api/v1/session/attach",
            serde_json::json!({
                "session_id": "sess_test",
                "thread_id": "thread_resume_target",
                "client_id": "client_web",
                "lease_seconds": 180
            }),
            "session.attach",
        ),
        (
            "/api/v1/session/sess_test/attachment/renew",
            serde_json::json!({
                "client_id": "client_web",
                "lease_seconds": 60
            }),
            "attachment.renew",
        ),
        (
            "/api/v1/session/sess_test/attachment/release",
            serde_json::json!({
                "client_id": "client_web"
            }),
            "attachment.release",
        ),
    ];

    for (path, body_json, operation_kind) in post_cases {
        let response = route_request(
            &post_json_request(path, body_json),
            &sample_snapshot(),
            &new_command_queue(),
            None,
        );
        assert_eq!(
            response.status, 200,
            "expected POST contract success for {path}"
        );
        let body = json_body(&response.body);
        assert_json_path_eq(&body, "session.id", "sess_test", path);
        assert_json_path_eq(&body, "session.scope", "process", path);
        assert_json_path_eq(&body, "operation.kind", operation_kind, path);
        assert_eq!(
            body["accepted"],
            Value::Bool(true),
            "expected accepted=true for {path}"
        );
    }
}
