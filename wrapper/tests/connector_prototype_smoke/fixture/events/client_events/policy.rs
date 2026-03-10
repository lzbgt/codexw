use super::*;

#[test]
fn broker_client_fixture_enforces_client_event_ownership_and_observer_reads() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_client_event_policy");
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
                                "session_id": "sess_client_event_policy",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 120,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 5233445566u64
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
                        "/api/v1/session/sess_client_event_policy/events"
                    );
                    let event_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 400\n",
                        "event: service.updated\n",
                        "data: {\"session_id\":\"sess_client_event_policy\",\"job_id\":\"bg-1\",\"alias\":\"dev.api\",\"capabilities\":[\"@frontend.pending\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(event_stream.as_bytes())
                        .context("write initial policy event stream")?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_policy/client_event"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse rival client-event body")?;
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
                                        "lease_expires_at_ms": 5233445566u64,
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
                        "/api/v1/session/sess_client_event_policy/client_event"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse owner client-event body")?;
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["event"], "selection.changed");
                    assert_eq!(body["data"]["selection"], "services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session_id": "sess_client_event_policy",
                            "client_id": "fixture-owner",
                            "event": "selection.changed",
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
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_policy/services/bg-1"
                    );
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
                                "label": "frontend",
                                "ready_state": "ready"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_client_event_policy/events"
                    );
                    assert_eq!(
                        request._headers.get("last-event-id").map(String::as_str),
                        Some("400")
                    );
                    let resumed_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 401\n",
                        "event: client.event\n",
                        "data: {\"session_id\":\"sess_client_event_policy\",\"client_id\":\"fixture-owner\",\"event\":\"selection.changed\",\"data\":{\"selection\":\"services\"}}\n",
                        "\n"
                    );
                    stream
                        .write_all(resumed_stream.as_bytes())
                        .context("write resumed policy event stream")?;
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
        "thread_client_event_policy",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_client_event_policy"
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
        "sess_client_event_policy",
        "--limit",
        "1",
    ])?;
    let observer_events_json: Value =
        serde_json::from_str(&observer_events_output).context("parse observer events output")?;
    assert_eq!(observer_events_json["status"], 200);
    assert_eq!(observer_events_json["body"][0]["event"], "service.updated");

    let rival_publish_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "client-event",
        "--session-id",
        "sess_client_event_policy",
        "--event",
        "selection.changed",
        "--data-json",
        "{\"selection\":\"services\"}",
    ])?;
    let rival_publish_json: Value =
        serde_json::from_str(&rival_publish_output).context("parse rival publish output")?;
    assert_eq!(rival_publish_json["status"], 409);
    assert_eq!(
        rival_publish_json["body"]["error"]["code"],
        "attachment_conflict"
    );
    assert_eq!(
        rival_publish_json["body"]["error"]["details"]["requested_client_id"],
        "fixture-rival"
    );
    assert_eq!(
        rival_publish_json["body"]["error"]["details"]["current_attachment"]["client_id"],
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
        "sess_client_event_policy",
        "--event",
        "selection.changed",
        "--data-json",
        "{\"selection\":\"services\"}",
    ])?;
    let owner_publish_json: Value =
        serde_json::from_str(&owner_publish_output).context("parse owner publish output")?;
    assert_eq!(owner_publish_json["status"], 200);
    assert_eq!(
        owner_publish_json["body"]["operation"]["kind"],
        "client.event"
    );
    assert_eq!(owner_publish_json["body"]["data"]["selection"], "services");

    let observer_detail_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "service-detail",
        "--session-id",
        "sess_client_event_policy",
        "--job-ref",
        "bg-1",
    ])?;
    let observer_detail_json: Value =
        serde_json::from_str(&observer_detail_output).context("parse observer detail output")?;
    assert_eq!(observer_detail_json["status"], 200);
    assert_eq!(observer_detail_json["body"]["service"]["alias"], "dev.api");

    let observer_resume_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "events",
        "--session-id",
        "sess_client_event_policy",
        "--last-event-id",
        "400",
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
        "selection.changed"
    );
    assert_eq!(
        observer_resume_json["body"][0]["data"]["data"]["data"]["selection"],
        "services"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
