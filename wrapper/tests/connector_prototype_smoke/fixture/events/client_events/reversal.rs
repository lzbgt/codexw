use super::*;

#[test]
fn broker_client_fixture_handles_repeated_client_event_role_reversal() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_client_event_reversal");
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
                                "session_id": "sess_client_event_reversal",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 120,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 7233445566u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_reversal/events"
                    );
                    let initial_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 900\n",
                        "event: service.updated\n",
                        "data: {\"session_id\":\"sess_client_event_reversal\",\"job_id\":\"bg-1\",\"alias\":\"dev.api\",\"capabilities\":[\"@frontend.pending\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(initial_stream.as_bytes())
                        .context("write initial client-event reversal stream")?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_reversal/client_event"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse rival pre-release client-event body")?;
                    assert_eq!(body["client_id"], "fixture-rival");
                    assert_eq!(body["event"], "selection.changed");
                    assert_eq!(body["data"]["selection"], "services");
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
                                        "lease_expires_at_ms": 7233445566u64,
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
                        "/api/v1/session/sess_client_event_reversal/attachment/release"
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
                                "session_id": "sess_client_event_reversal",
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
                        "/api/v1/session/sess_client_event_reversal/attachment/renew"
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
                                "session_id": "sess_client_event_reversal",
                                "attachment": {
                                    "client_id": "fixture-rival",
                                    "lease_seconds": 90,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 7233446666u64
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
                        "/api/v1/session/sess_client_event_reversal/client_event"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse rival post-renew client-event body")?;
                    assert_eq!(body["client_id"], "fixture-rival");
                    assert_eq!(body["event"], "selection.confirmed");
                    assert_eq!(body["data"]["selection"], "services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session_id": "sess_client_event_reversal",
                            "client_id": "fixture-rival",
                            "event": "selection.confirmed",
                            "data": {
                                "selection": "services"
                            },
                            "operation": {
                                "kind": "client.event"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_reversal/client_event"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse former-owner client-event body")?;
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["event"], "selection.owner_return");
                    assert_eq!(body["data"]["selection"], "services");
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
                                        "lease_expires_at_ms": 7233446666u64,
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
                        "/api/v1/session/sess_client_event_reversal/attachment/release"
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
                                "session_id": "sess_client_event_reversal",
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
                        "/api/v1/session/sess_client_event_reversal/attachment/renew"
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
                                "session_id": "sess_client_event_reversal",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 60,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 7233447777u64
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
                        "/api/v1/session/sess_client_event_reversal/client_event"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse owner post-retake client-event body")?;
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["event"], "selection.owner_return");
                    assert_eq!(body["data"]["selection"], "services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session_id": "sess_client_event_reversal",
                            "client_id": "fixture-owner",
                            "event": "selection.owner_return",
                            "data": {
                                "selection": "services"
                            },
                            "operation": {
                                "kind": "client.event"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                10 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_reversal/events"
                    );
                    assert_eq!(
                        request._headers.get("last-event-id").map(String::as_str),
                        Some("900")
                    );
                    let resumed_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 901\n",
                        "event: client.event\n",
                        "data: {\"session_id\":\"sess_client_event_reversal\",\"client_id\":\"fixture-owner\",\"event\":\"selection.owner_return\",\"data\":{\"selection\":\"services\"}}\n",
                        "\n"
                    );
                    stream
                        .write_all(resumed_stream.as_bytes())
                        .context("write resumed client-event reversal stream")?;
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
        "thread_client_event_reversal",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_client_event_reversal"
    );

    let observer_events_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "events",
        "--session-id",
        "sess_client_event_reversal",
        "--limit",
        "1",
    ])?;
    let observer_events_json: Value =
        serde_json::from_str(&observer_events_output).context("parse observer events output")?;
    assert_eq!(observer_events_json["status"], 200);
    assert_eq!(observer_events_json["body"][0]["event"], "service.updated");

    let rival_pre_release_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "client-event",
        "--session-id",
        "sess_client_event_reversal",
        "--event",
        "selection.changed",
        "--data-json",
        "{\"selection\":\"services\"}",
    ])?;
    let rival_pre_release_json: Value = serde_json::from_str(&rival_pre_release_output)
        .context("parse rival pre-release output")?;
    assert_eq!(rival_pre_release_json["status"], 409);
    assert_eq!(
        rival_pre_release_json["body"]["error"]["code"],
        "attachment_conflict"
    );
    assert_eq!(
        rival_pre_release_json["body"]["error"]["details"]["current_attachment"]["client_id"],
        "fixture-owner"
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
        "sess_client_event_reversal",
    ])?;
    let owner_release_json: Value =
        serde_json::from_str(&owner_release_output).context("parse owner release output")?;
    assert_eq!(owner_release_json["status"], 200);
    assert!(
        !owner_release_json["body"]["session"]["attachment"]["lease_active"]
            .as_bool()
            .unwrap_or(true)
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
        "sess_client_event_reversal",
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

    let rival_publish_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "client-event",
        "--session-id",
        "sess_client_event_reversal",
        "--event",
        "selection.confirmed",
        "--data-json",
        "{\"selection\":\"services\"}",
    ])?;
    let rival_publish_json: Value =
        serde_json::from_str(&rival_publish_output).context("parse rival publish output")?;
    assert_eq!(rival_publish_json["status"], 200);
    assert_eq!(rival_publish_json["body"]["client_id"], "fixture-rival");

    let former_owner_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-owner",
        "client-event",
        "--session-id",
        "sess_client_event_reversal",
        "--event",
        "selection.owner_return",
        "--data-json",
        "{\"selection\":\"services\"}",
    ])?;
    let former_owner_json: Value =
        serde_json::from_str(&former_owner_output).context("parse former owner output")?;
    assert_eq!(former_owner_json["status"], 409);
    assert_eq!(
        former_owner_json["body"]["error"]["details"]["current_attachment"]["client_id"],
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
        "sess_client_event_reversal",
    ])?;
    let rival_release_json: Value =
        serde_json::from_str(&rival_release_output).context("parse rival release output")?;
    assert_eq!(rival_release_json["status"], 200);
    assert!(
        !rival_release_json["body"]["session"]["attachment"]["lease_active"]
            .as_bool()
            .unwrap_or(true)
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
        "sess_client_event_reversal",
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

    let owner_publish_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-owner",
        "client-event",
        "--session-id",
        "sess_client_event_reversal",
        "--event",
        "selection.owner_return",
        "--data-json",
        "{\"selection\":\"services\"}",
    ])?;
    let owner_publish_json: Value =
        serde_json::from_str(&owner_publish_output).context("parse owner publish output")?;
    assert_eq!(owner_publish_json["status"], 200);
    assert_eq!(owner_publish_json["body"]["client_id"], "fixture-owner");

    let observer_resume_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "events",
        "--session-id",
        "sess_client_event_reversal",
        "--last-event-id",
        "900",
        "--limit",
        "1",
    ])?;
    let observer_resume_json: Value =
        serde_json::from_str(&observer_resume_output).context("parse observer resume output")?;
    assert_eq!(observer_resume_json["status"], 200);
    assert_eq!(observer_resume_json["body"][0]["event"], "client.event");
    assert_eq!(
        observer_resume_json["body"][0]["data"]["data"]["client_id"],
        "fixture-owner"
    );
    assert_eq!(
        observer_resume_json["body"][0]["data"]["data"]["event"],
        "selection.owner_return"
    );
    assert_eq!(
        observer_resume_json["body"][0]["data"]["data"]["data"]["selection"],
        "services"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
