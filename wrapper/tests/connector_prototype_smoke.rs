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
    headers: HashMap<String, String>,
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
        headers,
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
