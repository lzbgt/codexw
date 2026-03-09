use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use serde_json::json;

const READ_TIMEOUT: Duration = Duration::from_secs(5);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

struct ChildGuard {
    child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Debug)]
struct ParsedRequest {
    method: String,
    path: String,
    _headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[test]
fn connector_alias_attach_projects_session_and_lease_headers() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/session/attach");

        let body: Value = serde_json::from_slice(&request.body).context("parse forwarded body")?;
        assert_eq!(body["session_id"], "sess_1");
        assert_eq!(body["thread_id"], "thread_1");
        assert_eq!(body["client_id"], "remote-web");
        assert_eq!(body["lease_seconds"], 45);

        write_http_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/json")],
            serde_json::to_vec(&json!({
                "ok": true,
                "session": {
                    "session_id": "sess_1",
                    "attachment": {
                        "client_id": "remote-web",
                        "lease_seconds": 45
                    }
                }
            }))?
            .as_slice(),
        )?;
        Ok(())
    });

    let connector_port = reserve_port()?;
    let _connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(connector_port)?;

    let body = "{\"thread_id\":\"thread_1\"}";
    let request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/attach HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 45\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    let response = send_raw_request(connector_port, &request)?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("\"session_id\":\"sess_1\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_session_create_and_attachment_lifecycle_routes_work() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..3 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_1");
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "attachment": {
                                    "client_id": "remote-web",
                                    "lease_seconds": 45,
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/attachment/renew");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse renew body")?;
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 90);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "attachment": {
                                    "client_id": "remote-web",
                                    "lease_seconds": 90,
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/attachment/release");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse release body")?;
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "attachment": Value::Null,
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    });

    let connector_port = reserve_port()?;
    let _connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(connector_port)?;

    let create_body = "{\"thread_id\":\"thread_1\"}";
    let create_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 45\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        create_body.len(),
        create_body
    );
    let create_response = send_raw_request(connector_port, &create_request)?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(create_response.contains("\"session_id\":\"sess_1\""));

    let renew_body = "{}";
    let renew_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/attachment/renew HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 90\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        renew_body.len(),
        renew_body
    );
    let renew_response = send_raw_request(connector_port, &renew_request)?;
    assert!(renew_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(renew_response.contains("\"lease_seconds\":90"));

    let release_body = "{}";
    let release_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/attachment/release HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        release_body.len(),
        release_body
    );
    let release_response = send_raw_request(connector_port, &release_request)?;
    assert!(release_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(release_response.contains("\"attachment\":null"));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_events_route_wraps_broker_metadata() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/v1/session/sess_1/events");

        let body = concat!(
            "id: 7\n",
            "event: turn.updated\n",
            "data: {\"session_id\":\"sess_1\",\"status\":\"running\"}\n",
            "\n"
        );
        write_http_response(
            &mut stream,
            200,
            "OK",
            &[
                ("Content-Type", "text/event-stream"),
                ("Cache-Control", "no-cache"),
            ],
            body.as_bytes(),
        )?;
        let _ = stream.shutdown(Shutdown::Both);
        Ok(())
    });

    let connector_port = reserve_port()?;
    let _connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/events HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("Content-Type: text/event-stream"));
    assert!(response.contains("id: 7\n"));
    assert!(response.contains("event: turn.updated\n"));
    assert!(response.contains("\"source\":\"codexw\""));
    assert!(response.contains("\"agent_id\":\"codexw-lab\""));
    assert!(response.contains("\"deployment_id\":\"mac-mini-01\""));
    assert!(response.contains("\"session_id\":\"sess_1\""));
    assert!(response.contains("\"status\":\"running\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_events_route_forwards_last_event_id() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/v1/session/sess_1/events");
        assert_eq!(
            request._headers.get("last-event-id").map(String::as_str),
            Some("42")
        );

        let body = concat!(
            ": heartbeat\n",
            "id: 43\n",
            "event: transcript.updated\n",
            "data: {\"session_id\":\"sess_1\",\"items\":2}\n",
            "\n"
        );
        write_http_response(
            &mut stream,
            200,
            "OK",
            &[
                ("Content-Type", "text/event-stream"),
                ("Cache-Control", "no-cache"),
            ],
            body.as_bytes(),
        )?;
        let _ = stream.shutdown(Shutdown::Both);
        Ok(())
    });

    let connector_port = reserve_port()?;
    let _connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/events HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Last-Event-ID: 42\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("Content-Type: text/event-stream"));
    assert!(response.contains(": heartbeat\n"));
    assert!(response.contains("id: 43\n"));
    assert!(response.contains("event: transcript.updated\n"));
    assert!(response.contains("\"source\":\"codexw\""));
    assert!(response.contains("\"agent_id\":\"codexw-lab\""));
    assert!(response.contains("\"deployment_id\":\"mac-mini-01\""));
    assert!(response.contains("\"session_id\":\"sess_1\""));
    assert!(response.contains("\"items\":2"));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_shell_start_projects_client_and_lease_headers() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/session/sess_1/shells/start");

        let body: Value = serde_json::from_slice(&request.body).context("parse forwarded body")?;
        assert_eq!(body["command"], "sleep 5");
        assert_eq!(body["intent"], "observation");
        assert_eq!(body["client_id"], "remote-term");
        assert_eq!(body["lease_seconds"], 30);

        write_http_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/json")],
            serde_json::to_vec(&json!({
                "ok": true,
                "job": {
                    "job_id": "bg-1",
                    "intent": "observation"
                }
            }))?
            .as_slice(),
        )?;
        Ok(())
    });

    let connector_port = reserve_port()?;
    let _connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(connector_port)?;

    let body = "{\"command\":\"sleep 5\",\"intent\":\"observation\"}";
    let request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/shells HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-term\r\n",
            "X-Codexw-Lease-Seconds: 30\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    let response = send_raw_request(connector_port, &request)?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("\"job_id\":\"bg-1\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_service_run_maps_to_local_service_route() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api/run");

        let body: Value = serde_json::from_slice(&request.body).context("parse forwarded body")?;
        assert_eq!(body["recipe"], "health");
        assert_eq!(body["client_id"], "mobile-ios");

        write_http_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/json")],
            serde_json::to_vec(&json!({
                "ok": true,
                "service": {
                    "job_id": "dev.api",
                    "label": "API"
                },
                "recipe": {
                    "name": "health"
                },
                "result": "healthy"
            }))?
            .as_slice(),
        )?;
        Ok(())
    });

    let connector_port = reserve_port()?;
    let _connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(connector_port)?;

    let body = "{\"recipe\":\"health\"}";
    let request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: mobile-ios\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    let response = send_raw_request(connector_port, &request)?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("\"name\":\"health\""));
    assert!(response.contains("\"result\":\"healthy\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_broker_style_workflow_covers_turn_transcript_and_orchestration() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..5 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_1");
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "thread_id": "thread_1",
                                "attachment": {
                                    "client_id": "remote-web",
                                    "lease_seconds": 45
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/turn/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse turn body")?;
                    assert_eq!(body["prompt"], "Summarize the repository status");
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "turn": {
                                "status": "submitted"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/transcript");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "items": [
                                {
                                    "role": "user",
                                    "text": "Summarize the repository status"
                                },
                                {
                                    "role": "assistant",
                                    "text": "Repository is clean and connector alias coverage is expanding."
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/orchestration/status");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "status": {
                                "main_agent": "runnable",
                                "next_action": "Inspect transcript or workers"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/orchestration/dependencies"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "dependencies": [
                                {
                                    "from": "main",
                                    "to": "shell:bg-1",
                                    "state": "sidecar"
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    });

    let connector_port = reserve_port()?;
    let _connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(connector_port)?;

    let create_body = "{\"thread_id\":\"thread_1\"}";
    let create_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 45\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        create_body.len(),
        create_body
    );
    let create_response = send_raw_request(connector_port, &create_request)?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(create_response.contains("\"session_id\":\"sess_1\""));

    let turn_body = "{\"prompt\":\"Summarize the repository status\"}";
    let turn_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/turns HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        turn_body.len(),
        turn_body
    );
    let turn_response = send_raw_request(connector_port, &turn_request)?;
    assert!(turn_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(turn_response.contains("\"submitted\""));

    let transcript_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/transcript HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(transcript_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(transcript_response.contains("Summarize the repository status"));
    assert!(transcript_response.contains("connector alias coverage is expanding"));

    let orchestration_status_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/orchestration/status HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(orchestration_status_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(orchestration_status_response.contains("\"main_agent\":\"runnable\""));

    let dependency_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/orchestration/dependencies HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(dependency_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(dependency_response.contains("\"to\":\"shell:bg-1\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_broker_style_workflow_covers_shell_and_service_control() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..7 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_1");
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "thread_id": "thread_1",
                                "attachment": {
                                    "client_id": "remote-web",
                                    "lease_seconds": 45
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/shells/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse shell start body")?;
                    assert_eq!(body["command"], "sleep 5");
                    assert_eq!(body["intent"], "observation");
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "job": {
                                "job_id": "bg-1",
                                "intent": "observation"
                            },
                            "interaction": {
                                "operation": "start"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "services": [
                                {
                                    "job_id": "dev.api",
                                    "label": "API",
                                    "ready_state": "ready"
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/attach"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse attach body")?;
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "dev.api",
                                "label": "API"
                            },
                            "interaction": {
                                "operation": "attach"
                            },
                            "attachment": "curl http://127.0.0.1:8080/health"
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api/wait");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse wait body")?;
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["timeout_ms"], 5000);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "dev.api",
                                "ready_state": "ready"
                            },
                            "interaction": {
                                "operation": "wait"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api/run");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse run body")?;
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["recipe"], "health");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "dev.api",
                                "label": "API"
                            },
                            "interaction": {
                                "operation": "run"
                            },
                            "recipe": {
                                "name": "health"
                            },
                            "result": "healthy"
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/capabilities");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "capabilities": [
                                {
                                    "capability": "@api.http",
                                    "status": "healthy",
                                    "providers": ["dev.api"]
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    });

    let connector_port = reserve_port()?;
    let _connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(connector_port)?;

    let create_body = "{\"thread_id\":\"thread_1\"}";
    let create_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 45\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        create_body.len(),
        create_body
    );
    let create_response = send_raw_request(connector_port, &create_request)?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(create_response.contains("\"session_id\":\"sess_1\""));

    let shell_body = "{\"command\":\"sleep 5\",\"intent\":\"observation\"}";
    let shell_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/shells HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 45\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        shell_body.len(),
        shell_body
    );
    let shell_response = send_raw_request(connector_port, &shell_request)?;
    assert!(shell_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(shell_response.contains("\"job_id\":\"bg-1\""));

    let services_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/services HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(services_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(services_response.contains("\"job_id\":\"dev.api\""));

    let attach_body = "{}";
    let attach_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/services/dev.api/attach HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        attach_body.len(),
        attach_body
    );
    let attach_response = send_raw_request(connector_port, &attach_request)?;
    assert!(attach_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(attach_response.contains("curl http://127.0.0.1:8080/health"));

    let wait_body = "{\"timeout_ms\":5000}";
    let wait_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/services/dev.api/wait HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        wait_body.len(),
        wait_body
    );
    let wait_response = send_raw_request(connector_port, &wait_request)?;
    assert!(wait_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(wait_response.contains("\"ready_state\":\"ready\""));

    let run_body = "{\"recipe\":\"health\"}";
    let run_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        run_body.len(),
        run_body
    );
    let run_response = send_raw_request(connector_port, &run_request)?;
    assert!(run_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(run_response.contains("\"name\":\"health\""));
    assert!(run_response.contains("\"result\":\"healthy\""));

    let capabilities_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/capabilities HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(capabilities_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(capabilities_response.contains("\"capability\":\"@api.http\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_broker_style_workflow_covers_service_mutations() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..7 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_1");
                    assert_eq!(body["client_id"], "remote-admin");
                    assert_eq!(body["lease_seconds"], 60);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "thread_id": "thread_1",
                                "attachment": {
                                    "client_id": "remote-admin",
                                    "lease_seconds": 60
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/provide"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse provide body")?;
                    assert_eq!(body["client_id"], "remote-admin");
                    assert_eq!(body["capabilities"], json!(["@api.http", "@api.health"]));
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "dev.api",
                                "capabilities": ["@api.http", "@api.health"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/depend"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse depend body")?;
                    assert_eq!(body["client_id"], "remote-admin");
                    assert_eq!(body["dependsOnCapabilities"], json!(["@db.primary"]));
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "dev.api",
                                "depends_on_capabilities": ["@db.primary"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/contract"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse contract body")?;
                    assert_eq!(body["client_id"], "remote-admin");
                    assert_eq!(body["label"], "Public API");
                    assert_eq!(body["endpoint"], "http://127.0.0.1:8080");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "dev.api",
                                "label": "Public API",
                                "endpoint": "http://127.0.0.1:8080"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/relabel"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse relabel body")?;
                    assert_eq!(body["client_id"], "remote-admin");
                    assert_eq!(body["label"], "Prod API");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "dev.api",
                                "label": "Prod API"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "services": [
                                {
                                    "job_id": "dev.api",
                                    "label": "Prod API",
                                    "capabilities": ["@api.http", "@api.health"],
                                    "depends_on_capabilities": ["@db.primary"],
                                    "endpoint": "http://127.0.0.1:8080"
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/capabilities");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "capabilities": [
                                {
                                    "capability": "@api.http",
                                    "status": "healthy",
                                    "providers": ["dev.api"]
                                },
                                {
                                    "capability": "@db.primary",
                                    "status": "missing",
                                    "consumers": ["dev.api"]
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    });

    let connector_port = reserve_port()?;
    let _connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(connector_port)?;

    let create_body = "{\"thread_id\":\"thread_1\"}";
    let create_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-admin\r\n",
            "X-Codexw-Lease-Seconds: 60\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        create_body.len(),
        create_body
    );
    let create_response = send_raw_request(connector_port, &create_request)?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(create_response.contains("\"session_id\":\"sess_1\""));

    let provide_body = "{\"capabilities\":[\"@api.http\",\"@api.health\"]}";
    let provide_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/services/dev.api/provide HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-admin\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        provide_body.len(),
        provide_body
    );
    let provide_response = send_raw_request(connector_port, &provide_request)?;
    assert!(provide_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(provide_response.contains("\"@api.health\""));

    let depend_body = "{\"dependsOnCapabilities\":[\"@db.primary\"]}";
    let depend_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/services/dev.api/depend HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-admin\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        depend_body.len(),
        depend_body
    );
    let depend_response = send_raw_request(connector_port, &depend_request)?;
    assert!(depend_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(depend_response.contains("\"@db.primary\""));

    let contract_body = "{\"label\":\"Public API\",\"endpoint\":\"http://127.0.0.1:8080\"}";
    let contract_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/services/dev.api/contract HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-admin\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        contract_body.len(),
        contract_body
    );
    let contract_response = send_raw_request(connector_port, &contract_request)?;
    assert!(contract_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(contract_response.contains("\"Public API\""));
    assert!(contract_response.contains("http://127.0.0.1:8080"));

    let relabel_body = "{\"label\":\"Prod API\"}";
    let relabel_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/services/dev.api/relabel HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-admin\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        relabel_body.len(),
        relabel_body
    );
    let relabel_response = send_raw_request(connector_port, &relabel_request)?;
    assert!(relabel_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(relabel_response.contains("\"Prod API\""));

    let services_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/services HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(services_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(services_response.contains("\"label\":\"Prod API\""));
    assert!(services_response.contains("\"@api.health\""));
    assert!(services_response.contains("\"@db.primary\""));

    let capabilities_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/capabilities HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(capabilities_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(capabilities_response.contains("\"capability\":\"@api.http\""));
    assert!(capabilities_response.contains("\"capability\":\"@db.primary\""));
    assert!(capabilities_response.contains("\"status\":\"missing\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

fn spawn_connector(port: u16, local_api_port: u16) -> Result<ChildGuard> {
    let binary = connector_binary()?;
    let child = Command::new(binary)
        .arg("--bind")
        .arg(format!("127.0.0.1:{port}"))
        .arg("--local-api-base")
        .arg(format!("http://127.0.0.1:{local_api_port}"))
        .arg("--agent-id")
        .arg("codexw-lab")
        .arg("--deployment-id")
        .arg("mac-mini-01")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn connector prototype")?;
    Ok(ChildGuard { child })
}

fn connector_binary() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_codexw-connector-prototype") {
        return Ok(PathBuf::from(path));
    }

    let current_exe = std::env::current_exe().context("resolve current test executable")?;
    let debug_dir = current_exe
        .parent()
        .and_then(|path| path.parent())
        .context("resolve cargo target debug directory")?;
    let mut fallback = debug_dir.join("codexw-connector-prototype");
    if cfg!(windows) {
        fallback.set_extension("exe");
    }
    if fallback.exists() {
        return Ok(fallback);
    }

    anyhow::bail!("resolve connector prototype test binary")
}

fn reserve_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("bind ephemeral port")?;
    let port = listener.local_addr().context("ephemeral addr")?.port();
    drop(listener);
    Ok(port)
}

fn wait_for_healthz(port: u16) -> Result<()> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        if Instant::now() > deadline {
            anyhow::bail!("connector healthz did not become reachable");
        }
        match send_raw_request(
            port,
            concat!(
                "GET /healthz HTTP/1.1\r\n",
                "Host: localhost\r\n",
                "Connection: close\r\n",
                "\r\n"
            ),
        ) {
            Ok(response) if response.starts_with("HTTP/1.1 200 OK\r\n") => return Ok(()),
            _ => thread::sleep(POLL_INTERVAL),
        }
    }
}

fn send_raw_request(port: u16, request: &str) -> Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .with_context(|| format!("connect to 127.0.0.1:{port}"))?;
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set client read timeout")?;
    stream
        .write_all(request.as_bytes())
        .context("write raw request")?;
    let _ = stream.shutdown(Shutdown::Write);
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .context("read raw response")?;
    String::from_utf8(bytes).context("decode raw response")
}

fn read_http_request(stream: &mut TcpStream) -> Result<ParsedRequest> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .context("set fake local api read timeout")?;
    let mut buffer = [0_u8; 1024];
    let mut request_bytes = Vec::new();
    let header_end = loop {
        let read = stream
            .read(&mut buffer)
            .context("read fake local api request")?;
        if read == 0 {
            anyhow::bail!("request closed before headers");
        }
        request_bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = request_bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            break index + 4;
        }
    };
    let request_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let mut lines = request_text.split("\r\n");
    let request_line = lines.next().context("missing request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().context("missing request method")?.to_string();
    let path = parts.next().context("missing request path")?.to_string();
    let _version = parts.next().context("missing request version")?;

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = request_bytes[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut buffer).context("read request body")?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
    }
    body.truncate(content_length);

    Ok(ParsedRequest {
        method,
        path,
        _headers: headers,
        body,
    })
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Result<()> {
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    stream
        .write_all(response.as_bytes())
        .context("write fake local api head")?;
    stream
        .write_all(body)
        .context("write fake local api body")?;
    Ok(())
}
