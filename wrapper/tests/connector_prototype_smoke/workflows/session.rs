use super::*;

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
                                "main_agent_state": "runnable",
                                "waits": 0
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
                                    "to": "agent:sub-1",
                                    "kind": "wait",
                                    "blocking": false
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
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let client = BrokerClient::new(connector_port, "codexw-lab");
    let create_response = client.create_session(
        "{\"thread_id\":\"thread_1\"}",
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
            ("X-Codexw-Lease-Seconds", "45"),
        ],
    )?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(create_response.contains("\"session_id\":\"sess_1\""));

    let turn_response = client.session_request(
        "POST",
        "sess_1",
        "/turns",
        Some("{\"prompt\":\"Summarize the repository status\"}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(turn_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(turn_response.contains("\"submitted\""));

    let transcript_response = client.session_request("GET", "sess_1", "/transcript", None, &[])?;
    assert!(transcript_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(transcript_response.contains("Summarize the repository status"));
    assert!(transcript_response.contains("Repository is clean"));

    let orchestration_status_response =
        client.session_request("GET", "sess_1", "/orchestration/status", None, &[])?;
    assert!(orchestration_status_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(orchestration_status_response.contains("\"main_agent_state\":\"runnable\""));

    let dependencies_response =
        client.session_request("GET", "sess_1", "/orchestration/dependencies", None, &[])?;
    assert!(dependencies_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(dependencies_response.contains("\"from\":\"main\""));
    assert!(dependencies_response.contains("\"to\":\"agent:sub-1\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
