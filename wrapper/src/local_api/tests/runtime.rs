use super::fixture::get_request;
use super::fixture::json_body;
use super::fixture::new_command_queue;
use super::fixture::route_request;
use super::fixture::sample_snapshot;
use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;

#[test]
fn runtime_route_requires_auth_when_token_is_configured() {
    let response = route_request(
        &get_request("/api/v1/runtime"),
        &sample_snapshot(),
        &new_command_queue(),
        Some("secret"),
    );
    assert_eq!(response.status, 401);
    assert_eq!(json_body(&response.body)["error"]["code"], "unauthorized");
}

#[test]
fn runtime_route_exposes_broker_and_mobile_discovery_metadata() {
    let response = route_request(
        &get_request("/api/v1/runtime"),
        &sample_snapshot(),
        &new_command_queue(),
        None,
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![(
            HEADER_LOCAL_API_VERSION.to_string(),
            CODEXW_LOCAL_API_VERSION.to_string(),
        )]
    );
    let body = json_body(&response.body);
    assert_eq!(body["local_api_version"], CODEXW_LOCAL_API_VERSION);
    assert_eq!(body["runtime"]["instance_id"], "inst_test");
    assert_eq!(body["runtime"]["suggested_deployment_id"], "codexw-m2-lab");
    assert_eq!(body["runtime"]["host_os"], "macos");
    assert_eq!(body["runtime"]["host_arch"], "aarch64");
    assert_eq!(body["runtime"]["apple_silicon"], true);
    assert_eq!(body["runtime"]["preferred_broker_transport"], "connector");
    assert_eq!(body["runtime"]["recommended_remote_clients"][0], "ios");
    assert_eq!(body["session_id"], "sess_test");
}
