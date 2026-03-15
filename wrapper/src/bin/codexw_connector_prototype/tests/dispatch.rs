use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

use serde_json::Value;

use crate::Cli;
use crate::handle_connection;

fn sample_cli() -> Cli {
    Cli {
        bind: "127.0.0.1:0".to_string(),
        local_api_base: "http://127.0.0.1:8080".to_string(),
        local_api_token: Some("secret".to_string()),
        connector_token: Some("connector".to_string()),
        agent_id: "codexw-lab".to_string(),
        deployment_id: "mac-mini-01".to_string(),
    }
}

fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let client = TcpStream::connect(addr).expect("connect");
    let (server, _) = listener.accept().expect("accept");
    (client, server)
}

fn run_connection(raw_request: &'static [u8], cli: &Cli) -> String {
    let (mut client, server) = connected_pair();
    let cli = cli.clone();
    let writer = thread::spawn(move || {
        client.write_all(raw_request).expect("write request");
        let _ = client.shutdown(std::net::Shutdown::Write);
        let mut bytes = Vec::new();
        client.read_to_end(&mut bytes).expect("read response");
        String::from_utf8(bytes).expect("utf8")
    });
    handle_connection(server, &cli).expect("handle connection");
    writer.join().expect("client thread")
}

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

    assert!(response.starts_with("HTTP/1.1 403 Error\r\n"));
    let body = response.split("\r\n\r\n").nth(1).expect("body");
    let json: Value = serde_json::from_str(body).expect("json");
    assert_eq!(json["error"]["code"], "route_not_allowed");
    assert_eq!(
        json["error"]["details"]["local_path"],
        "/api/v1/session/sess_1/internal/debug"
    );
}
