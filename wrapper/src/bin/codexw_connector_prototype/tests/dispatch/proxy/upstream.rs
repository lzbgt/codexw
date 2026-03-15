use serde_json::Value;

use super::super::run_connection;
use super::super::sample_cli;

#[test]
fn handle_connection_surfaces_validation_errors_for_invalid_proxy_body() {
    let cli = sample_cli();
    let response = run_connection(
        b"POST /v1/agents/codexw-lab/proxy/api/v1/session/new HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer connector\r\nX-Codexw-Lease-Seconds: nope\r\nContent-Length: 0\r\n\r\n",
        &cli,
    );

    assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    let body = response.split("\r\n\r\n").nth(1).expect("body");
    let json: Value = serde_json::from_str(body).expect("json");
    assert_eq!(json["error"]["code"], "validation_error");
    assert_eq!(json["error"]["details"]["field"], "x-codexw-lease-seconds");
}

#[test]
fn handle_connection_surfaces_upstream_unavailable_for_valid_proxy_request() {
    let mut cli = sample_cli();
    cli.local_api_base = "http://127.0.0.1:1".to_string();
    let response = run_connection(
        b"GET /v1/agents/codexw-lab/proxy/api/v1/session HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer connector\r\nContent-Length: 0\r\n\r\n",
        &cli,
    );

    assert!(response.starts_with("HTTP/1.1 502 Bad Gateway\r\n"));
    let body = response.split("\r\n\r\n").nth(1).expect("body");
    let json: Value = serde_json::from_str(body).expect("json");
    assert_eq!(json["error"]["code"], "upstream_unavailable");
}
