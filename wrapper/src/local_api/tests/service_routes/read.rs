use super::super::get_request;
use super::super::json_body;
use super::super::new_command_queue;
use super::super::route_request;
use super::super::sample_snapshot;

#[test]
fn service_detail_route_returns_structured_service() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/services/dev.frontend"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["service"]["id"], "bg-1");
    assert_eq!(body["service"]["alias"], "dev.frontend");
    assert_eq!(body["service"]["service_capabilities"][0], "@frontend.dev");
}

#[test]
fn capability_detail_route_returns_structured_capability() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/capabilities/@frontend.dev"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    let body = json_body(&response.body);
    assert_eq!(body["capability"]["capability"], "@frontend.dev");
    assert_eq!(body["capability"]["providers"][0]["job_id"], "bg-1");
    assert_eq!(body["capability"]["consumers"][0]["job_id"], "bg-2");
}

#[test]
fn capability_detail_route_returns_not_found_for_unknown_capability() {
    let response = route_request(
        &get_request("/api/v1/session/sess_test/capabilities/@missing.service"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 404);
    assert_eq!(
        json_body(&response.body)["error"]["code"],
        "capability_not_found"
    );
}
