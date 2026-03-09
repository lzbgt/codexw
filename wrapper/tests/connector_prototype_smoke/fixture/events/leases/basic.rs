use super::*;

#[test]
fn broker_client_fixture_handles_multi_client_lease_and_event_resume() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_multi_client_events");
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
                                "session_id": "sess_multi_client_events",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 120,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 2233445566u64
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
                        "/api/v1/session/sess_multi_client_events/events"
                    );
                    let event_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 80\n",
                        "event: service.updated\n",
                        "data: {\"session_id\":\"sess_multi_client_events\",\"job_id\":\"bg-1\",\"alias\":\"dev.api\",\"capabilities\":[\"@frontend.pending\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(event_stream.as_bytes())
                        .context("write initial multi-client event stream")?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_multi_client_events/services/bg-1/provide"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse rival provide body")?;
                    assert_eq!(body["client_id"], "fixture-rival");
                    assert_eq!(body["capabilities"], json!(["@frontend.dev"]));
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
                                        "lease_expires_at_ms": 2233445566u64,
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
                        "/api/v1/session/sess_multi_client_events/services/bg-1/provide"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse owner provide body")?;
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["capabilities"], json!(["@frontend.dev"]));
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
                                "capabilities": ["@frontend.dev"],
                                "ready_state": "ready"
                            },
                            "operation": {
                                "kind": "service.provide"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_multi_client_events/services/bg-1"
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
                                "capabilities": ["@frontend.dev"],
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
                        "/api/v1/session/sess_multi_client_events/events"
                    );
                    assert_eq!(
                        request._headers.get("last-event-id").map(String::as_str),
                        Some("80")
                    );
                    let resumed_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 81\n",
                        "event: capabilities.updated\n",
                        "data: {\"session_id\":\"sess_multi_client_events\",\"capability\":\"@frontend.dev\",\"status\":\"healthy\",\"providers\":[\"bg-1\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(resumed_stream.as_bytes())
                        .context("write resumed multi-client event stream")?;
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
        "thread_multi_client_events",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_multi_client_events"
    );

    let initial_events_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "events",
        "--session-id",
        "sess_multi_client_events",
        "--limit",
        "1",
    ])?;
    let initial_events_json: Value =
        serde_json::from_str(&initial_events_output).context("parse initial events output")?;
    assert_eq!(initial_events_json["status"], 200);
    assert_eq!(initial_events_json["body"][0]["event"], "service.updated");
    assert_eq!(
        initial_events_json["body"][0]["data"]["data"]["capabilities"][0],
        "@frontend.pending"
    );

    let rival_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "service-provide",
        "--session-id",
        "sess_multi_client_events",
        "--job-ref",
        "bg-1",
        "--values-json",
        "[\"@frontend.dev\"]",
    ])?;
    let rival_json: Value = serde_json::from_str(&rival_output).context("parse rival output")?;
    assert_eq!(rival_json["status"], 409);
    assert_eq!(rival_json["body"]["error"]["code"], "attachment_conflict");
    assert_eq!(
        rival_json["body"]["error"]["details"]["requested_client_id"],
        "fixture-rival"
    );
    assert_eq!(
        rival_json["body"]["error"]["details"]["current_attachment"]["client_id"],
        "fixture-owner"
    );

    let owner_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-owner",
        "service-provide",
        "--session-id",
        "sess_multi_client_events",
        "--job-ref",
        "bg-1",
        "--values-json",
        "[\"@frontend.dev\"]",
    ])?;
    let owner_json: Value = serde_json::from_str(&owner_output).context("parse owner output")?;
    assert_eq!(owner_json["status"], 200);
    assert_eq!(owner_json["body"]["operation"]["kind"], "service.provide");
    assert_eq!(
        owner_json["body"]["service"]["capabilities"][0],
        "@frontend.dev"
    );

    let detail_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "service-detail",
        "--session-id",
        "sess_multi_client_events",
        "--job-ref",
        "bg-1",
    ])?;
    let detail_json: Value = serde_json::from_str(&detail_output).context("parse detail output")?;
    assert_eq!(detail_json["status"], 200);
    assert_eq!(
        detail_json["body"]["service"]["capabilities"][0],
        "@frontend.dev"
    );

    let resumed_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "events",
        "--session-id",
        "sess_multi_client_events",
        "--last-event-id",
        "80",
        "--limit",
        "1",
    ])?;
    let resumed_json: Value =
        serde_json::from_str(&resumed_output).context("parse resumed output")?;
    assert_eq!(resumed_json["status"], 200);
    assert_eq!(resumed_json["body"][0]["event"], "capabilities.updated");
    assert_eq!(
        resumed_json["body"][0]["data"]["data"]["capability"],
        "@frontend.dev"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
