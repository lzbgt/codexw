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
fn broker_client_fixture_interrupts_turn_through_connector() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_interrupt");
                    assert_eq!(body["client_id"], "fixture-interrupt");
                    assert_eq!(body["lease_seconds"], 40);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_interrupt",
                                "attachment": {
                                    "client_id": "fixture-interrupt",
                                    "lease_seconds": 40
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
                        "/api/v1/session/sess_interrupt/turn/interrupt"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse interrupt body")?;
                    assert_eq!(body["client_id"], "fixture-interrupt");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "turn": {
                                "interrupted": true
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

    let base_url;
    {
        let connector_port = reserve_port()?;
        let mut connector = spawn_connector(connector_port, local_addr.port())?;
        wait_for_healthz(&mut connector, connector_port)?;
        base_url = format!("http://127.0.0.1:{connector_port}");

        let create_output = run_broker_client(&[
            "--base-url",
            &base_url,
            "--agent-id",
            "codexw-lab",
            "--client-id",
            "fixture-interrupt",
            "--lease-seconds",
            "40",
            "session-create",
            "--thread-id",
            "thread_interrupt",
        ])?;
        let create_json: Value =
            serde_json::from_str(&create_output).context("parse create output")?;
        assert_eq!(create_json["status"], 200);

        let interrupt_output = run_broker_client(&[
            "--base-url",
            &base_url,
            "--agent-id",
            "codexw-lab",
            "--client-id",
            "fixture-interrupt",
            "turn-interrupt",
            "--session-id",
            "sess_interrupt",
        ])?;
        let interrupt_json: Value =
            serde_json::from_str(&interrupt_output).context("parse interrupt output")?;
        assert_eq!(interrupt_json["status"], 200);
        assert_eq!(interrupt_json["body"]["turn"]["interrupted"], true);
    }

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
