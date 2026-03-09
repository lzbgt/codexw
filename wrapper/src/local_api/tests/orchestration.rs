use super::get_request;
use super::json_body;
use super::new_command_queue;
use super::route_request;
use super::sample_snapshot;

#[test]
fn orchestration_status_route_returns_structured_counts() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/orchestration/status"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["orchestration"]["main_agent_state"], "blocked");
    assert_eq!(body["orchestration"]["background_shell_job_count"], 2);
}

#[test]
fn orchestration_dependencies_route_returns_edges() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/orchestration/dependencies"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["dependencies"][0]["from"], "main");
    assert_eq!(
        body["dependencies"][0]["blocking"],
        serde_json::Value::Bool(true)
    );
}

#[test]
fn orchestration_workers_route_returns_live_and_cached_workers() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/orchestration/workers"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(
        body["workers"]["cached_agent_threads"][0]["id"],
        "thread_worker"
    );
    assert_eq!(body["workers"]["background_shells"][0]["id"], "bg-1");
}

#[test]
fn shells_services_and_capabilities_routes_return_filtered_views() {
    let snapshot = sample_snapshot();

    let shells = route_request(
        &get_request("/api/v1/session/sess_test/shells"),
        &snapshot,
        &new_command_queue(),
        None,
    );
    assert_eq!(shells.status, 200);
    assert_eq!(
        json_body(&shells.body)["shells"].as_array().map(Vec::len),
        Some(2)
    );

    let services = route_request(
        &get_request("/api/v1/session/sess_test/services"),
        &snapshot,
        &new_command_queue(),
        None,
    );
    assert_eq!(services.status, 200);
    let services_body = json_body(&services.body);
    assert_eq!(services_body["services"].as_array().map(Vec::len), Some(1));
    assert_eq!(services_body["services"][0]["intent"], "service");

    let capabilities = route_request(
        &get_request("/api/v1/session/sess_test/capabilities"),
        &snapshot,
        &new_command_queue(),
        None,
    );
    assert_eq!(capabilities.status, 200);
    let capabilities_body = json_body(&capabilities.body);
    assert_eq!(
        capabilities_body["capabilities"][0]["capability"],
        "@frontend.dev"
    );
    assert_eq!(
        capabilities_body["capabilities"][0]["providers"][0]["job_id"],
        "bg-1"
    );
}

#[test]
fn transcript_route_returns_semantic_conversation_entries() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/transcript"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["transcript"].as_array().map(Vec::len), Some(2));
    assert_eq!(body["transcript"][0]["role"], "user");
    assert_eq!(body["transcript"][1]["role"], "assistant");
}
