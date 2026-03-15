use super::super::LocalApiCommand;
use super::super::json_body;
use super::super::new_command_queue;
use super::super::post_json_request;
use super::super::route_request;
use super::super::sample_snapshot;

#[test]
fn service_update_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/update",
            serde_json::json!({
                "jobId": "bg-1",
                "capabilities": ["frontend.dev"],
                "client_id": "client_web"
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateService {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "capabilities": ["frontend.dev"],
                "client_id": "client_web"
            }),
        })
    );
}

#[test]
fn service_provide_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/dev.frontend/provide",
            serde_json::json!({ "capabilities": ["frontend.dev"], "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateService {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "capabilities": ["frontend.dev"],
                "client_id": "client_web"
            }),
        })
    );
}

#[test]
fn service_contract_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/contract",
            serde_json::json!({
                "endpoint": "http://127.0.0.1:3001",
                "readyPattern": "listening",
                "client_id": "client_web",
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateService {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "endpoint": "http://127.0.0.1:3001",
                "readyPattern": "listening",
                "client_id": "client_web",
            }),
        })
    );
}

#[test]
fn service_relabel_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/@frontend.dev/relabel",
            serde_json::json!({ "label": "frontend service", "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateService {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-1",
                "label": "frontend service",
                "client_id": "client_web"
            }),
        })
    );
}

#[test]
fn dependency_update_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/dependencies/update",
            serde_json::json!({
                "jobId": "bg-2",
                "dependsOnCapabilities": ["frontend.dev"],
                "client_id": "client_web"
            }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateDependencies {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-2",
                "dependsOnCapabilities": ["frontend.dev"],
                "client_id": "client_web"
            }),
        })
    );
}

#[test]
fn service_depend_route_enqueues_local_api_command() {
    let queue = new_command_queue();
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-2/depend",
            serde_json::json!({ "dependsOnCapabilities": ["frontend.dev"], "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &queue,
        None,
    );
    assert_eq!(response.status, 200);
    let queued = queue.lock().expect("queue");
    assert_eq!(
        queued.front(),
        Some(&LocalApiCommand::UpdateDependencies {
            session_id: "sess_test".to_string(),
            arguments: serde_json::json!({
                "jobId": "bg-2",
                "dependsOnCapabilities": ["frontend.dev"],
                "client_id": "client_web"
            }),
        })
    );
}

#[test]
fn service_contract_route_requires_contract_fields() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/contract",
            serde_json::json!({ "client_id": "client_web" }),
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
fn service_update_route_rejects_conflicting_attachment_client() {
    let response = route_request(
        &post_json_request(
            "/api/v1/session/sess_test/services/update",
            serde_json::json!({
                "jobId": "bg-1",
                "capabilities": ["frontend.dev"],
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
