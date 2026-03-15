use serde_json::Value;

use super::super::run_connection;
use super::super::sample_cli;

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
