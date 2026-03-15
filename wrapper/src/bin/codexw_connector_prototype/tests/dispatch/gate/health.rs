use serde_json::Value;

use super::super::run_connection;
use super::super::sample_cli;

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
