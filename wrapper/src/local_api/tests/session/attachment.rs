use super::super::LocalApiCommand;
use super::super::json_body;
use super::super::new_command_queue;
use super::super::post_json_request;
use super::super::route_request;
use super::super::sample_snapshot;

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
