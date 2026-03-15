use serde_json::Value;

use super::run_connection;
use super::sample_cli;

#[test]
fn handle_connection_serves_healthz_without_auth() {
    let cli = sample_cli();
    let response = run_connection(
        b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n",
        &cli,
    );

    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    let body = response.split("\r\n\r\n").nth(1).expect("body");
    let json: Value = serde_json::from_str(body).expect("json");
    assert_eq!(json["ok"], true);
    assert_eq!(json["agent_id"], "codexw-lab");
    assert_eq!(json["deployment_id"], "mac-mini-01");
}

#[test]
fn handle_connection_rejects_missing_connector_bearer_token() {
    let cli = sample_cli();
    let response = run_connection(
        b"GET /v1/agents/codexw-lab/sessions HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n",
        &cli,
    );

    assert!(response.starts_with("HTTP/1.1 401 Unauthorized\r\n"));
    let body = response.split("\r\n\r\n").nth(1).expect("body");
    let json: Value = serde_json::from_str(body).expect("json");
    assert_eq!(json["error"]["code"], "unauthorized");
}

#[test]
fn handle_connection_rejects_unknown_connector_route() {
    let cli = sample_cli();
    let response = run_connection(
        b"GET /v1/agents/codexw-lab/unknown HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer connector\r\nContent-Length: 0\r\n\r\n",
        &cli,
    );

    assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
    let body = response.split("\r\n\r\n").nth(1).expect("body");
    let json: Value = serde_json::from_str(body).expect("json");
    assert_eq!(json["error"]["code"], "not_found");
}

#[test]
fn handle_connection_rejects_proxy_targets_outside_allowed_surface() {
    let cli = sample_cli();
    let response = run_connection(
        b"GET /v1/agents/codexw-lab/proxy/api/v1/session/sess_1/internal/debug HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer connector\r\nContent-Length: 0\r\n\r\n",
        &cli,
    );

    assert!(response.starts_with("HTTP/1.1 403 Forbidden\r\n"));
    let body = response.split("\r\n\r\n").nth(1).expect("body");
    let json: Value = serde_json::from_str(body).expect("json");
    assert_eq!(json["error"]["code"], "route_not_allowed");
    assert_eq!(
        json["error"]["details"]["local_path"],
        "/api/v1/session/sess_1/internal/debug"
    );
}
