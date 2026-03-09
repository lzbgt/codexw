use super::*;

#[test]
fn broker_client_fixture_handles_repeated_lease_role_reversal() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..11 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_role_reversal");
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["lease_seconds"], 120);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_role_reversal",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 120,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 4233445566u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_role_reversal/events");
                    let initial_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 200\n",
                        "event: service.updated\n",
                        "data: {\"session_id\":\"sess_role_reversal\",\"job_id\":\"bg-1\",\"alias\":\"dev.api\",\"capabilities\":[\"@frontend.pending\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(initial_stream.as_bytes())
                        .context("write initial role-reversal event stream")?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_role_reversal/services/bg-1/provide"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse rival pre-release provide body")?;
                    assert_eq!(body["client_id"], "fixture-rival");
                    assert_eq!(body["capabilities"], json!(["@frontend.rival"]));
                    write_http_response(
                        &mut stream,
                        409,
                        "Conflict",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": false,
                            "error": {
                                "status": 409,
                                "code": "attachment_conflict",
                                "message": "active attachment lease blocks this mutation",
                                "retryable": false,
                                "details": {
                                    "requested_client_id": "fixture-rival",
                                    "current_attachment": {
                                        "client_id": "fixture-owner",
                                        "lease_seconds": 120,
                                        "lease_expires_at_ms": 4233445566u64,
                                        "lease_active": true
                                    }
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
                        "/api/v1/session/sess_role_reversal/attachment/release"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse owner release body")?;
                    assert_eq!(body["client_id"], "fixture-owner");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_role_reversal",
                                "attachment": {
                                    "client_id": Value::Null,
                                    "lease_seconds": Value::Null,
                                    "lease_active": false,
                                    "lease_expires_at_ms": Value::Null
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_role_reversal/attachment/renew"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse rival renew body")?;
                    assert_eq!(body["client_id"], "fixture-rival");
                    assert_eq!(body["lease_seconds"], 90);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_role_reversal",
                                "attachment": {
                                    "client_id": "fixture-rival",
                                    "lease_seconds": 90,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 4233446666u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_role_reversal/services/bg-1/provide"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse rival post-renew provide body")?;
                    assert_eq!(body["client_id"], "fixture-rival");
                    assert_eq!(body["capabilities"], json!(["@frontend.rival"]));
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "alias": "dev.api",
                                "capabilities": ["@frontend.rival"],
                                "ready_state": "ready"
                            },
                            "operation": {
                                "kind": "service.provide"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_role_reversal/services/bg-1/provide"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse former-owner provide body")?;
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["capabilities"], json!(["@frontend.owner_return"]));
                    write_http_response(
                        &mut stream,
                        409,
                        "Conflict",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": false,
                            "error": {
                                "status": 409,
                                "code": "attachment_conflict",
                                "message": "active attachment lease blocks this mutation",
                                "retryable": false,
                                "details": {
                                    "requested_client_id": "fixture-owner",
                                    "current_attachment": {
                                        "client_id": "fixture-rival",
                                        "lease_seconds": 90,
                                        "lease_expires_at_ms": 4233446666u64,
                                        "lease_active": true
                                    }
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                7 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_role_reversal/attachment/release"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse rival release body")?;
                    assert_eq!(body["client_id"], "fixture-rival");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_role_reversal",
                                "attachment": {
                                    "client_id": Value::Null,
                                    "lease_seconds": Value::Null,
                                    "lease_active": false,
                                    "lease_expires_at_ms": Value::Null
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                8 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_role_reversal/attachment/renew"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse owner renew body")?;
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["lease_seconds"], 60);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_role_reversal",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 60,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 4233447777u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                9 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_role_reversal/services/bg-1/provide"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse owner post-retake provide body")?;
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["capabilities"], json!(["@frontend.owner_return"]));
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "alias": "dev.api",
                                "capabilities": ["@frontend.owner_return"],
                                "ready_state": "ready"
                            },
                            "operation": {
                                "kind": "service.provide"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                10 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_role_reversal/events");
                    assert_eq!(
                        request._headers.get("last-event-id").map(String::as_str),
                        Some("200")
                    );
                    let resumed_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 201\n",
                        "event: capabilities.updated\n",
                        "data: {\"session_id\":\"sess_role_reversal\",\"capability\":\"@frontend.owner_return\",\"status\":\"healthy\",\"providers\":[\"bg-1\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(resumed_stream.as_bytes())
                        .context("write role-reversal resumed event stream")?;
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
        "fixture-owner",
        "--lease-seconds",
        "120",
        "session-create",
        "--thread-id",
        "thread_role_reversal",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_role_reversal"
    );

    let observer_events = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "events",
        "--session-id",
        "sess_role_reversal",
        "--limit",
        "1",
    ])?;
    let observer_json: Value =
        serde_json::from_str(&observer_events).context("parse observer events")?;
    assert_eq!(observer_json["status"], 200);
    assert_eq!(observer_json["body"][0]["event"], "service.updated");

    let rival_conflict_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "service-provide",
        "--session-id",
        "sess_role_reversal",
        "--job-ref",
        "bg-1",
        "--values-json",
        "[\"@frontend.rival\"]",
    ])?;
    let rival_conflict_json: Value =
        serde_json::from_str(&rival_conflict_output).context("parse rival conflict output")?;
    assert_eq!(rival_conflict_json["status"], 409);
    assert_eq!(
        rival_conflict_json["body"]["error"]["code"],
        "attachment_conflict"
    );

    let owner_release_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-owner",
        "attachment-release",
        "--session-id",
        "sess_role_reversal",
    ])?;
    let owner_release_json: Value =
        serde_json::from_str(&owner_release_output).context("parse owner release output")?;
    assert_eq!(owner_release_json["status"], 200);
    assert_eq!(
        owner_release_json["body"]["session"]["attachment"]["lease_active"],
        false
    );

    let rival_renew_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "attachment-renew",
        "--session-id",
        "sess_role_reversal",
        "--lease-seconds",
        "90",
    ])?;
    let rival_renew_json: Value =
        serde_json::from_str(&rival_renew_output).context("parse rival renew output")?;
    assert_eq!(rival_renew_json["status"], 200);
    assert_eq!(
        rival_renew_json["body"]["session"]["attachment"]["client_id"],
        "fixture-rival"
    );

    let rival_success_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "service-provide",
        "--session-id",
        "sess_role_reversal",
        "--job-ref",
        "bg-1",
        "--values-json",
        "[\"@frontend.rival\"]",
    ])?;
    let rival_success_json: Value =
        serde_json::from_str(&rival_success_output).context("parse rival success output")?;
    assert_eq!(rival_success_json["status"], 200);
    assert_eq!(
        rival_success_json["body"]["service"]["capabilities"][0],
        "@frontend.rival"
    );

    let former_owner_conflict_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-owner",
        "service-provide",
        "--session-id",
        "sess_role_reversal",
        "--job-ref",
        "bg-1",
        "--values-json",
        "[\"@frontend.owner_return\"]",
    ])?;
    let former_owner_conflict_json: Value = serde_json::from_str(&former_owner_conflict_output)
        .context("parse former owner conflict output")?;
    assert_eq!(former_owner_conflict_json["status"], 409);
    assert_eq!(
        former_owner_conflict_json["body"]["error"]["details"]["current_attachment"]["client_id"],
        "fixture-rival"
    );

    let rival_release_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "attachment-release",
        "--session-id",
        "sess_role_reversal",
    ])?;
    let rival_release_json: Value =
        serde_json::from_str(&rival_release_output).context("parse rival release output")?;
    assert_eq!(rival_release_json["status"], 200);
    assert_eq!(
        rival_release_json["body"]["session"]["attachment"]["lease_active"],
        false
    );

    let owner_renew_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-owner",
        "attachment-renew",
        "--session-id",
        "sess_role_reversal",
        "--lease-seconds",
        "60",
    ])?;
    let owner_renew_json: Value =
        serde_json::from_str(&owner_renew_output).context("parse owner renew output")?;
    assert_eq!(owner_renew_json["status"], 200);
    assert_eq!(
        owner_renew_json["body"]["session"]["attachment"]["client_id"],
        "fixture-owner"
    );

    let owner_success_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-owner",
        "service-provide",
        "--session-id",
        "sess_role_reversal",
        "--job-ref",
        "bg-1",
        "--values-json",
        "[\"@frontend.owner_return\"]",
    ])?;
    let owner_success_json: Value =
        serde_json::from_str(&owner_success_output).context("parse owner success output")?;
    assert_eq!(owner_success_json["status"], 200);
    assert_eq!(
        owner_success_json["body"]["service"]["capabilities"][0],
        "@frontend.owner_return"
    );

    let observer_resume = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "events",
        "--session-id",
        "sess_role_reversal",
        "--last-event-id",
        "200",
        "--limit",
        "1",
    ])?;
    let observer_resume_json: Value =
        serde_json::from_str(&observer_resume).context("parse observer resume")?;
    assert_eq!(observer_resume_json["status"], 200);
    assert_eq!(
        observer_resume_json["body"][0]["data"]["data"]["capability"],
        "@frontend.owner_return"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
