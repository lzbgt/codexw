use super::*;

#[test]
fn node_broker_client_fixture_drives_connector_session_workflow() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..4 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_node");
                    assert_eq!(body["client_id"], "node-fixture");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_node",
                                "thread_id": "thread_node",
                                "attachment": {
                                    "client_id": "node-fixture",
                                    "lease_seconds": 45,
                                    "lease_active": true
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_node/turn/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse turn body")?;
                    assert_eq!(body["prompt"], "Summarize connector status");
                    assert_eq!(body["client_id"], "node-fixture");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "accepted": true,
                            "turn": {
                                "state": "running"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_node/transcript");
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
                                    "text": "Summarize connector status"
                                },
                                {
                                    "role": "assistant",
                                    "text": "Connector status summarized."
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_node/orchestration/status"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "status": {
                                "main_agent_state": "runnable",
                                "next_action": "Continue the main turn"
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
    let create_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-fixture",
        "--lease-seconds",
        "45",
        "session-create",
        "--thread-id",
        "thread_node",
    ])?;
    let create_json: Value =
        serde_json::from_str(&create_output).context("parse node create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(create_json["body"]["session"]["session_id"], "sess_node");

    let turn_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-fixture",
        "turn-start",
        "--session-id",
        "sess_node",
        "--prompt",
        "Summarize connector status",
    ])?;
    let turn_json: Value = serde_json::from_str(&turn_output).context("parse node turn output")?;
    assert_eq!(turn_json["status"], 200);
    assert_eq!(turn_json["body"]["accepted"], true);

    let transcript_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "transcript",
        "--session-id",
        "sess_node",
    ])?;
    let transcript_json: Value =
        serde_json::from_str(&transcript_output).context("parse node transcript output")?;
    assert_eq!(transcript_json["status"], 200);
    assert_eq!(
        transcript_json["body"]["items"][1]["text"],
        "Connector status summarized."
    );

    let orchestration_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "orchestration-status",
        "--session-id",
        "sess_node",
    ])?;
    let orchestration_json: Value =
        serde_json::from_str(&orchestration_output).context("parse node orchestration output")?;
    assert_eq!(orchestration_json["status"], 200);
    assert_eq!(
        orchestration_json["body"]["status"]["main_agent_state"],
        "runnable"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn node_broker_client_fixture_drives_connector_attachment_lifecycle_workflow() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_node_attach");
                    assert_eq!(body["client_id"], "node-attach");
                    assert_eq!(body["lease_seconds"], 30);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_node_attach",
                                "attachment": {
                                    "client_id": "node-attach",
                                    "lease_seconds": 30,
                                    "lease_active": true
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
                        "/api/v1/session/sess_node_attach/attachment/renew"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse renew body")?;
                    assert_eq!(body["client_id"], "node-attach");
                    assert_eq!(body["lease_seconds"], 90);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "attachment": {
                                "client_id": "node-attach",
                                "lease_seconds": 90,
                                "lease_active": true
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_node_attach");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_node_attach",
                                "attachment": {
                                    "client_id": "node-attach",
                                    "lease_seconds": 90,
                                    "lease_active": true
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_node_attach/attachment/release"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse release body")?;
                    assert_eq!(body["client_id"], "node-attach");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "attachment": {
                                "client_id": "node-attach",
                                "lease_seconds": 90,
                                "lease_active": false
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_node_attach");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_node_attach",
                                "attachment": {
                                    "client_id": "node-attach",
                                    "lease_seconds": 90,
                                    "lease_active": false
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
    let create_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-attach",
        "--lease-seconds",
        "30",
        "session-create",
        "--thread-id",
        "thread_node_attach",
    ])?;
    let create_json: Value =
        serde_json::from_str(&create_output).context("parse node create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_node_attach"
    );

    let renew_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-attach",
        "attachment-renew",
        "--session-id",
        "sess_node_attach",
        "--lease-seconds",
        "90",
    ])?;
    let renew_json: Value =
        serde_json::from_str(&renew_output).context("parse node renew output")?;
    assert_eq!(renew_json["status"], 200);
    assert_eq!(renew_json["body"]["attachment"]["lease_seconds"], 90);
    assert_eq!(renew_json["body"]["attachment"]["lease_active"], true);

    let session_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "session-get",
        "--session-id",
        "sess_node_attach",
    ])?;
    let session_json: Value =
        serde_json::from_str(&session_output).context("parse node session output")?;
    assert_eq!(session_json["status"], 200);
    assert_eq!(
        session_json["body"]["session"]["attachment"]["lease_seconds"],
        90
    );
    assert_eq!(
        session_json["body"]["session"]["attachment"]["lease_active"],
        true
    );

    let release_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-attach",
        "attachment-release",
        "--session-id",
        "sess_node_attach",
    ])?;
    let release_json: Value =
        serde_json::from_str(&release_output).context("parse node release output")?;
    assert_eq!(
        release_json["status"], 200,
        "node release output: {release_output}"
    );
    assert_eq!(release_json["body"]["attachment"]["lease_active"], false);

    let released_session_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "session-get",
        "--session-id",
        "sess_node_attach",
    ])?;
    let released_session_json: Value = serde_json::from_str(&released_session_output)
        .context("parse node released session output")?;
    assert_eq!(released_session_json["status"], 200);
    assert_eq!(
        released_session_json["body"]["session"]["attachment"]["lease_active"],
        false
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
