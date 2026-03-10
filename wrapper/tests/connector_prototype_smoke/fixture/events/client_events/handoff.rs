use super::*;

#[test]
fn broker_client_fixture_handles_client_event_lease_handoff_and_resume() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_client_event_handoff");
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
                                "session_id": "sess_client_event_handoff",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 120,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 6233445566u64
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
                        "/api/v1/session/sess_client_event_handoff/events"
                    );
                    let event_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 700\n",
                        "event: service.updated\n",
                        "data: {\"session_id\":\"sess_client_event_handoff\",\"job_id\":\"bg-1\",\"alias\":\"dev.api\",\"capabilities\":[\"@frontend.pending\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(event_stream.as_bytes())
                        .context("write initial handoff stream")?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_handoff/client_event"
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
                                        "lease_expires_at_ms": 6233445566u64,
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
                        "/api/v1/session/sess_client_event_handoff/attachment/release"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse release body")?;
                    assert_eq!(body["client_id"], "fixture-owner");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_client_event_handoff",
                                "attachment": Value::Null
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_handoff/attachment/renew"
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
                                "session_id": "sess_client_event_handoff",
                                "attachment": {
                                    "client_id": "fixture-rival",
                                    "lease_seconds": 90,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 6233446600u64
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
                        "/api/v1/session/sess_client_event_handoff/client_event"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse rival post-takeover client-event body")?;
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
                            "session_id": "sess_client_event_handoff",
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
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_handoff/events"
                    );
                    assert_eq!(
                        request._headers.get("last-event-id").map(String::as_str),
                        Some("700")
                    );
                    let resumed_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 701\n",
                        "event: client.event\n",
                        "data: {\"session_id\":\"sess_client_event_handoff\",\"client_id\":\"fixture-rival\",\"event\":\"selection.confirmed\",\"data\":{\"selection\":\"services\"}}\n",
                        "\n"
                    );
                    stream
                        .write_all(resumed_stream.as_bytes())
                        .context("write resumed handoff stream")?;
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
        "thread_client_event_handoff",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_client_event_handoff"
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
        "sess_client_event_handoff",
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
        "sess_client_event_handoff",
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

    let release_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-owner",
        "attachment-release",
        "--session-id",
        "sess_client_event_handoff",
    ])?;
    let release_json: Value =
        serde_json::from_str(&release_output).context("parse release output")?;
    assert_eq!(release_json["status"], 200);
    assert!(release_json["body"]["session"]["attachment"].is_null());

    let renew_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "attachment-renew",
        "--session-id",
        "sess_client_event_handoff",
        "--lease-seconds",
        "90",
    ])?;
    let renew_json: Value = serde_json::from_str(&renew_output).context("parse renew output")?;
    assert_eq!(renew_json["status"], 200);
    assert_eq!(
        renew_json["body"]["session"]["attachment"]["client_id"],
        "fixture-rival"
    );
    assert_eq!(
        renew_json["body"]["session"]["attachment"]["lease_seconds"],
        90
    );

    let rival_post_takeover_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "client-event",
        "--session-id",
        "sess_client_event_handoff",
        "--event",
        "selection.confirmed",
        "--data-json",
        "{\"selection\":\"services\"}",
    ])?;
    let rival_post_takeover_json: Value = serde_json::from_str(&rival_post_takeover_output)
        .context("parse rival post-takeover output")?;
    assert_eq!(rival_post_takeover_json["status"], 200);
    assert_eq!(
        rival_post_takeover_json["body"]["operation"]["kind"],
        "client.event"
    );
    assert_eq!(
        rival_post_takeover_json["body"]["client_id"],
        "fixture-rival"
    );

    let observer_resume_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "events",
        "--session-id",
        "sess_client_event_handoff",
        "--last-event-id",
        "700",
        "--limit",
        "1",
    ])?;
    let observer_resume_json: Value =
        serde_json::from_str(&observer_resume_output).context("parse observer resume output")?;
    assert_eq!(observer_resume_json["status"], 200);
    assert_eq!(observer_resume_json["body"][0]["event"], "client.event");
    assert_eq!(
        observer_resume_json["body"][0]["data"]["data"]["client_id"],
        "fixture-rival"
    );
    assert_eq!(
        observer_resume_json["body"][0]["data"]["data"]["event"],
        "selection.confirmed"
    );
    assert_eq!(
        observer_resume_json["body"][0]["data"]["data"]["data"]["selection"],
        "services"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
