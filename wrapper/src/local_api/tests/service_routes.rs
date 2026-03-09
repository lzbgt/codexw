use super::LocalApiCommand;
use super::json_body;
use super::new_command_queue;
use super::post_json_request;
use super::route_request;
use super::route_request_with_manager;
use super::sample_service_manager;
use super::sample_snapshot;

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
fn service_attach_route_returns_attachment_summary() {
    let manager = sample_service_manager();
    let response = route_request_with_manager(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/attach",
            serde_json::json!({ "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        &manager,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["interaction"]["kind"], "attach");
    assert_eq!(body["service"]["id"], "bg-1");
    assert_eq!(body["service"]["service_protocol"], "http");
    assert!(
        matches!(
            body["service"]["service_readiness"].as_str(),
            Some("ready") | Some("booting")
        ),
        "service readiness should be present in structured payload"
    );
    let _ = manager.terminate_all_running();
}

#[test]
fn service_wait_route_returns_ready_status() {
    let manager = sample_service_manager();
    let response = route_request_with_manager(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/wait",
            serde_json::json!({ "timeoutMs": 2000, "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        &manager,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    let result = body["result"]
        .as_str()
        .expect("wait result should be a string");
    assert!(result.contains("already ready") || result.contains("became ready"));
    assert!(result.contains("Ready pattern: READY"));
    assert_eq!(body["result_text"], body["result"]);
    assert_eq!(body["interaction"]["kind"], "wait");
    assert_eq!(body["interaction"]["timeout_ms"], 2000);
    assert_eq!(body["service"]["id"], "bg-1");
    assert_eq!(body["service"]["service_readiness"], "ready");
    let _ = manager.terminate_all_running();
}

#[test]
fn service_run_route_invokes_service_recipe() {
    let manager = sample_service_manager();
    let response = route_request_with_manager(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/run",
            serde_json::json!({ "recipe": "health", "client_id": "client_web" }),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        &manager,
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    let result = body["result"]
        .as_str()
        .expect("run result should be a string");
    assert!(result.contains("Invoked recipe `health`"));
    assert!(result.contains("Action: stdin \"status\""));
    assert_eq!(body["result_text"], body["result"]);
    assert_eq!(body["interaction"]["kind"], "run");
    assert_eq!(body["recipe"]["name"], "health");
    assert_eq!(body["recipe"]["args"], serde_json::Value::Null);
    assert_eq!(body["service"]["id"], "bg-1");
    assert_eq!(body["service"]["interaction_recipe_names"][0], "health");
    let _ = manager.terminate_all_running();
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
fn service_contract_route_requires_contract_field() {
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
fn service_attach_route_rejects_anonymous_request_when_lease_active() {
    let manager = sample_service_manager();
    let response = route_request_with_manager(
        &post_json_request(
            "/api/v1/session/sess_test/services/bg-1/attach",
            serde_json::json!({}),
        ),
        &sample_snapshot(),
        &new_command_queue(),
        &manager,
        None,
    );
    assert_eq!(response.status, 409);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "attachment_conflict"
    );
    let _ = manager.terminate_all_running();
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
