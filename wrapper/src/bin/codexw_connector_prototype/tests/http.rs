use std::collections::HashMap;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::adapter_contract::CODEXW_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::CODEXW_LOCAL_API_VERSION;
use crate::adapter_contract::HEADER_BROKER_ADAPTER_VERSION;
use crate::adapter_contract::HEADER_LOCAL_API_VERSION;

use super::super::Cli;
use super::super::http::from_upstream_response;
use super::super::http::json_error_response;
use super::super::http::json_ok_response;
use super::super::http::read_request;
use super::super::upstream::UpstreamResponse;

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

#[test]
fn json_ok_response_adds_adapter_version_header_and_body() {
    let response = json_ok_response(serde_json::json!({
        "ok": true,
        "session_id": "sess_1"
    }));
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string()
            ),
        ]
    );
    let body: Value = serde_json::from_slice(&response.body).expect("json body");
    assert_eq!(
        body["broker_adapter_version"],
        CODEXW_BROKER_ADAPTER_VERSION
    );
    assert_eq!(body["session_id"], "sess_1");
}

#[test]
fn json_error_response_adds_adapter_version_header_and_body() {
    let response = json_error_response(
        400,
        "validation_error",
        "bad lease header",
        Some(serde_json::json!({
            "field": "x-codexw-lease-seconds"
        })),
    );
    assert_eq!(response.status, 400);
    assert_eq!(
        response.headers,
        vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string()
            ),
        ]
    );
    let body: Value = serde_json::from_slice(&response.body).expect("json body");
    assert_eq!(body["ok"], false);
    assert_eq!(
        body["broker_adapter_version"],
        CODEXW_BROKER_ADAPTER_VERSION
    );
    assert_eq!(body["error"]["code"], "validation_error");
    assert_eq!(body["error"]["details"]["field"], "x-codexw-lease-seconds");
}

#[test]
fn from_upstream_response_adds_adapter_header_and_forwards_local_api_header() {
    let response = from_upstream_response(
        UpstreamResponse {
            status: 200,
            reason: "OK".to_string(),
            headers: HashMap::from([
                ("content-type".to_string(), "application/json".to_string()),
                (
                    "x-codexw-local-api-version".to_string(),
                    CODEXW_LOCAL_API_VERSION.to_string(),
                ),
            ]),
            body: br#"{"ok":true}"#.to_vec(),
        },
        &sample_cli(),
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Codexw-Agent-Id".to_string(), "codexw-lab".to_string()),
            (
                "X-Codexw-Deployment-Id".to_string(),
                "mac-mini-01".to_string()
            ),
            (
                HEADER_BROKER_ADAPTER_VERSION.to_string(),
                CODEXW_BROKER_ADAPTER_VERSION.to_string()
            ),
            (
                HEADER_LOCAL_API_VERSION.to_string(),
                CODEXW_LOCAL_API_VERSION.to_string()
            ),
        ]
    );
    let body: Value = serde_json::from_slice(&response.body).expect("json body");
    assert_eq!(body["ok"], true);
}

#[test]
fn read_request_tolerates_header_fragmentation_across_socket_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .expect("set read timeout");
        let request = read_request(&mut stream).expect("read fragmented request");
        (request.method, request.path, request.headers)
    });

    let mut client = TcpStream::connect(addr).expect("connect client");
    client
        .write_all(b"GET /v1/agents/codexw-lab/sessions/sess_1 HTTP/1.1\r\nHost: localhost\r\n")
        .expect("write first fragment");
    thread::sleep(Duration::from_millis(650));
    client
        .write_all(b"Connection: close\r\n\r\n")
        .expect("write second fragment");

    let (method, path, headers) = server.join().expect("join server");
    assert_eq!(method, "GET");
    assert_eq!(path, "/v1/agents/codexw-lab/sessions/sess_1");
    assert_eq!(headers.get("host").map(String::as_str), Some("localhost"));
}
