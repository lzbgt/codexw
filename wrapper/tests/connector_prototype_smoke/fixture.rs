use super::*;

#[test]
fn broker_client_fixture_drives_connector_session_workflow() -> Result<()> {
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
                    assert_eq!(body["client_id"], "fixture-web");
                    assert_eq!(body["lease_seconds"], 30);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "attachment": {
                                    "client_id": "fixture-web",
                                    "lease_seconds": 30
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
                    assert_eq!(body["client_id"], "fixture-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
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
                        json_bytes(json!({
                            "ok": true,
                            "items": [
                                {
                                    "role": "user",
                                    "text": "Summarize the repository status"
                                },
                                {
                                    "role": "assistant",
                                    "text": "Repository is clean."
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

    let base_url = format!("http://127.0.0.1:{connector_port}");
    let create_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-web",
        "--lease-seconds",
        "30",
        "session-create",
        "--thread-id",
        "thread_1",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(create_json["body"]["session"]["session_id"], "sess_1");

    let turn_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-web",
        "turn-start",
        "--session-id",
        "sess_1",
        "--prompt",
        "Summarize the repository status",
    ])?;
    let turn_json: Value = serde_json::from_str(&turn_output).context("parse turn output")?;
    assert_eq!(turn_json["status"], 200);
    assert_eq!(turn_json["body"]["turn"]["status"], "submitted");

    let transcript_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "transcript",
        "--session-id",
        "sess_1",
    ])?;
    let transcript_json: Value =
        serde_json::from_str(&transcript_output).context("parse transcript output")?;
    assert_eq!(transcript_json["status"], 200);
    assert_eq!(
        transcript_json["body"]["items"][0]["text"],
        "Summarize the repository status"
    );
    assert_eq!(
        transcript_json["body"]["items"][1]["text"],
        "Repository is clean."
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn broker_client_fixture_drives_connector_shell_service_workflow() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..6 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_shell");
                    assert_eq!(body["client_id"], "fixture-shell");
                    assert_eq!(body["lease_seconds"], 60);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_shell",
                                "attachment": {
                                    "client_id": "fixture-shell",
                                    "lease_seconds": 60
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_shell/shells/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse shell start body")?;
                    assert_eq!(body["command"], "npm run dev");
                    assert_eq!(body["label"], "frontend");
                    assert_eq!(body["client_id"], "fixture-shell");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "job": {
                                "job_id": "bg-1",
                                "label": "frontend",
                                "intent": "service"
                            },
                            "interaction": {
                                "kind": "shell_start",
                                "job_ref": "bg-1"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_shell/services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "services": [
                                {
                                    "job_id": "bg-1",
                                    "label": "frontend",
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
                        "/api/v1/session/sess_shell/services/bg-1/attach"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse attach body")?;
                    assert_eq!(body["client_id"], "fixture-shell");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "label": "frontend",
                                "ready_state": "ready"
                            },
                            "interaction": {
                                "kind": "attach",
                                "job_ref": "bg-1"
                            },
                            "attachment_text": "Open http://127.0.0.1:3000"
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_shell/services/bg-1/wait"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse wait body")?;
                    assert_eq!(body["timeout_ms"], 5000);
                    assert_eq!(body["client_id"], "fixture-shell");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "label": "frontend",
                                "ready_state": "ready"
                            },
                            "interaction": {
                                "kind": "wait",
                                "job_ref": "bg-1",
                                "timeout_ms": 5000
                            },
                            "ready": true
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_shell/services/bg-1/run");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse run body")?;
                    assert_eq!(body["recipe"], "health");
                    assert_eq!(body["client_id"], "fixture-shell");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "label": "frontend",
                                "ready_state": "ready"
                            },
                            "interaction": {
                                "kind": "run",
                                "job_ref": "bg-1"
                            },
                            "recipe": {
                                "name": "health"
                            },
                            "result_text": "healthy"
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

    let base_url = format!("http://127.0.0.1:{connector_port}");
    let create_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-shell",
        "--lease-seconds",
        "60",
        "session-create",
        "--thread-id",
        "thread_shell",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(create_json["body"]["session"]["session_id"], "sess_shell");

    let shell_start_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-shell",
        "shell-start",
        "--session-id",
        "sess_shell",
        "--shell-command",
        "npm run dev",
        "--intent",
        "service",
        "--label",
        "frontend",
    ])?;
    let shell_start_json: Value =
        serde_json::from_str(&shell_start_output).context("parse shell start output")?;
    assert_eq!(shell_start_json["status"], 200);
    assert_eq!(shell_start_json["body"]["job"]["job_id"], "bg-1");

    let services_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "services",
        "--session-id",
        "sess_shell",
    ])?;
    let services_json: Value =
        serde_json::from_str(&services_output).context("parse services output")?;
    assert_eq!(services_json["status"], 200);
    assert_eq!(services_json["body"]["services"][0]["label"], "frontend");

    let attach_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-shell",
        "service-attach",
        "--session-id",
        "sess_shell",
        "--job-ref",
        "bg-1",
    ])?;
    let attach_json: Value = serde_json::from_str(&attach_output).context("parse attach output")?;
    assert_eq!(attach_json["status"], 200);
    assert_eq!(attach_json["body"]["interaction"]["kind"], "attach");

    let wait_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-shell",
        "service-wait",
        "--session-id",
        "sess_shell",
        "--job-ref",
        "bg-1",
        "--timeout-ms",
        "5000",
    ])?;
    let wait_json: Value = serde_json::from_str(&wait_output).context("parse wait output")?;
    assert_eq!(wait_json["status"], 200);
    assert_eq!(wait_json["body"]["ready"], true);

    let run_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-shell",
        "service-run",
        "--session-id",
        "sess_shell",
        "--job-ref",
        "bg-1",
        "--recipe",
        "health",
    ])?;
    let run_json: Value = serde_json::from_str(&run_output).context("parse run output")?;
    assert_eq!(run_json["status"], 200);
    assert_eq!(run_json["body"]["recipe"]["name"], "health");
    assert_eq!(run_json["body"]["result_text"], "healthy");

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn broker_client_fixture_reports_attachment_conflict_details() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..2 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["client_id"], "lease-owner");
                    assert_eq!(body["lease_seconds"], 90);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_conflict",
                                "attachment": {
                                    "client_id": "lease-owner",
                                    "lease_seconds": 90,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 1234567890u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_conflict/turn/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse turn body")?;
                    assert_eq!(body["client_id"], "conflict-client");
                    write_http_response(
                        &mut stream,
                        409,
                        "Conflict",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "status": 409,
                            "error": {
                                "code": "attachment_conflict",
                                "message": "active attachment lease blocks this mutation",
                                "retryable": true,
                                "details": {
                                    "requested_client_id": "conflict-client",
                                    "current_attachment": {
                                        "client_id": "lease-owner",
                                        "lease_seconds": 90,
                                        "lease_expires_at_ms": 1234567890u64,
                                        "lease_active": true
                                    }
                                }
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
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let base_url = format!("http://127.0.0.1:{connector_port}");
    let create_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "lease-owner",
        "--lease-seconds",
        "90",
        "session-create",
        "--thread-id",
        "thread_conflict",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_conflict"
    );

    let turn_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "conflict-client",
        "turn-start",
        "--session-id",
        "sess_conflict",
        "--prompt",
        "This should conflict",
    ])?;
    let turn_json: Value = serde_json::from_str(&turn_output).context("parse turn output")?;
    assert_eq!(turn_json["status"], 409);
    assert_eq!(turn_json["body"]["error"]["code"], "attachment_conflict");
    assert_eq!(
        turn_json["body"]["error"]["details"]["requested_client_id"],
        "conflict-client"
    );
    assert_eq!(
        turn_json["body"]["error"]["details"]["current_attachment"]["client_id"],
        "lease-owner"
    );
    assert_eq!(
        turn_json["body"]["error"]["details"]["current_attachment"]["lease_active"],
        true
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
