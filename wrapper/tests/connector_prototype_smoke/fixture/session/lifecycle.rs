use super::*;

#[test]
fn broker_client_fixture_lists_sessions_through_connector() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/v1/session");
        write_http_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/json")],
            json_bytes(json!({
                "ok": true,
                "sessions": [
                    {
                        "session_id": "sess_list_1",
                        "thread_id": "thread_a"
                    },
                    {
                        "session_id": "sess_list_2",
                        "thread_id": "thread_b"
                    }
                ]
            }))?
            .as_slice(),
        )?;
        Ok(())
    });

    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let base_url = format!("http://127.0.0.1:{connector_port}");
    let output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "sessions",
    ])?;
    let output_json: Value = serde_json::from_str(&output).context("parse list output")?;
    assert_eq!(output_json["status"], 200);
    assert_eq!(
        output_json["body"]["sessions"][0]["session_id"],
        "sess_list_1"
    );
    assert_eq!(output_json["body"]["sessions"][1]["thread_id"], "thread_b");

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn broker_client_fixture_drives_connector_attachment_lifecycle_workflow() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_attach");
                    assert_eq!(body["client_id"], "fixture-attach");
                    assert_eq!(body["lease_seconds"], 30);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_attach",
                                "attachment": {
                                    "client_id": "fixture-attach",
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
                    assert_eq!(request.path, "/api/v1/session/sess_attach/attachment/renew");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse renew body")?;
                    assert_eq!(body["client_id"], "fixture-attach");
                    assert_eq!(body["lease_seconds"], 90);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "attachment": {
                                "client_id": "fixture-attach",
                                "lease_seconds": 90,
                                "lease_active": true
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_attach");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_attach",
                                "attachment": {
                                    "client_id": "fixture-attach",
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
                        "/api/v1/session/sess_attach/attachment/release"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse release body")?;
                    assert_eq!(body["client_id"], "fixture-attach");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "attachment": {
                                "client_id": "fixture-attach",
                                "lease_seconds": 90,
                                "lease_active": false
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_attach");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_attach",
                                "attachment": {
                                    "client_id": "fixture-attach",
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
    let create_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-attach",
        "--lease-seconds",
        "30",
        "session-create",
        "--thread-id",
        "thread_attach",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(create_json["body"]["session"]["session_id"], "sess_attach");

    let renew_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-attach",
        "attachment-renew",
        "--session-id",
        "sess_attach",
        "--lease-seconds",
        "90",
    ])?;
    let renew_json: Value = serde_json::from_str(&renew_output).context("parse renew output")?;
    assert_eq!(renew_json["status"], 200);
    assert_eq!(renew_json["body"]["attachment"]["lease_seconds"], 90);
    assert_eq!(renew_json["body"]["attachment"]["lease_active"], true);

    let session_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "session-get",
        "--session-id",
        "sess_attach",
    ])?;
    let session_json: Value =
        serde_json::from_str(&session_output).context("parse session output")?;
    assert_eq!(session_json["status"], 200);
    assert_eq!(
        session_json["body"]["session"]["attachment"]["lease_seconds"],
        90
    );
    assert_eq!(
        session_json["body"]["session"]["attachment"]["lease_active"],
        true
    );

    let release_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-attach",
        "attachment-release",
        "--session-id",
        "sess_attach",
    ])?;
    let release_json: Value =
        serde_json::from_str(&release_output).context("parse release output")?;
    assert_eq!(release_json["status"], 200);
    assert_eq!(release_json["body"]["attachment"]["lease_active"], false);

    let released_session_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "session-get",
        "--session-id",
        "sess_attach",
    ])?;
    let released_session_json: Value =
        serde_json::from_str(&released_session_output).context("parse released session output")?;
    assert_eq!(released_session_json["status"], 200);
    assert_eq!(
        released_session_json["body"]["session"]["attachment"]["lease_active"],
        false
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
