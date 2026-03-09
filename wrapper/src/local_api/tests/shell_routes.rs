use super::LocalApiCommand;
use super::json_body;
use super::new_command_queue;
use super::post_json_request;
use super::route_request;
use super::sample_snapshot;

#[test]
fn shell_start_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/start",
            serde_json::json!({ "command": "echo hi", "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["interaction"]["kind"], "shell.start");
    assert_eq!(body["interaction"]["queued"], true);
    assert_eq!(body["interaction"]["arguments"]["command"], "echo hi");
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::StartShell {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({ "command": "echo hi", "client_id": "client_web" }),
        })
    );
}

#[test]
fn shell_poll_route_returns_selected_shell_snapshot() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/bg-1/poll",
            serde_json::json!({}),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["interaction"]["kind"], "shell.poll");
    assert_eq!(body["interaction"]["shell_ref"], "bg-1");
    assert_eq!(body["shell"]["id"], "bg-1");
}

#[test]
fn shell_send_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/bg-1/send",
            serde_json::json!({ "text": "status", "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["shell"]["id"], "bg-1");
    assert_eq!(body["interaction"]["kind"], "shell.send");
    assert_eq!(body["interaction"]["shell_ref"], "bg-1");
    assert_eq!(body["interaction"]["text"], "status");
    assert_eq!(body["interaction"]["append_newline"], true);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::SendShellInput {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "text": "status",
                "appendNewline": true
            }),
        })
    );
}

#[test]
fn shell_terminate_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/bg-1/terminate",
            serde_json::json!({ "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["shell"]["id"], "bg-1");
    assert_eq!(body["interaction"]["kind"], "shell.terminate");
    assert_eq!(body["interaction"]["shell_ref"], "bg-1");
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::TerminateShell {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({ "jobId": "bg-1" }),
        })
    );
}

#[test]
fn shell_start_route_rejects_anonymous_request_when_lease_active() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/start",
            serde_json::json!({ "command": "echo hi" }),
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
fn shell_send_route_rejects_conflicting_attachment_client() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/shells/bg-1/send",
            serde_json::json!({ "text": "status", "client_id": "client_mobile" }),
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
