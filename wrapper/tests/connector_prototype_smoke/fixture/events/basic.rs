use super::*;

#[test]
fn broker_client_fixture_handles_events_and_focused_detail_resume() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_events");
                    assert_eq!(body["client_id"], "fixture-events");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_events",
                                "attachment": {
                                    "client_id": "fixture-events",
                                    "lease_seconds": 45
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_events/events");
                    let event_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 30\n",
                        "event: service.updated\n",
                        "data: {\"session_id\":\"sess_events\",\"job_id\":\"bg-1\",\"alias\":\"dev.api\"}\n",
                        "\n"
                    );
                    stream
                        .write_all(event_stream.as_bytes())
                        .context("write initial event stream")?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_events/services/bg-1");
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
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_events/capabilities/@frontend.dev"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "capability": {
                                "capability": "@frontend.dev",
                                "status": "healthy",
                                "providers": ["bg-1"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_events/events");
                    assert_eq!(
                        request._headers.get("last-event-id").map(String::as_str),
                        Some("30")
                    );
                    let resumed_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 31\n",
                        "event: capabilities.updated\n",
                        "data: {\"session_id\":\"sess_events\",\"capability\":\"@frontend.dev\",\"status\":\"healthy\",\"providers\":[\"bg-1\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(resumed_stream.as_bytes())
                        .context("write resumed event stream")?;
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
        "fixture-events",
        "--lease-seconds",
        "45",
        "session-create",
        "--thread-id",
        "thread_events",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(create_json["body"]["session"]["session_id"], "sess_events");

    let events_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "events",
        "--session-id",
        "sess_events",
        "--limit",
        "1",
    ])?;
    let events_json: Value = serde_json::from_str(&events_output).context("parse events output")?;
    assert_eq!(events_json["status"], 200);
    assert_eq!(events_json["body"][0]["event"], "service.updated");
    assert_eq!(events_json["body"][0]["data"]["data"]["alias"], "dev.api");

    let service_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "service-detail",
        "--session-id",
        "sess_events",
        "--job-ref",
        "bg-1",
    ])?;
    let service_json: Value =
        serde_json::from_str(&service_output).context("parse service detail output")?;
    assert_eq!(service_json["status"], 200);
    assert_eq!(service_json["body"]["service"]["alias"], "dev.api");

    let capability_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "capability-detail",
        "--session-id",
        "sess_events",
        "--capability",
        "@frontend.dev",
    ])?;
    let capability_json: Value =
        serde_json::from_str(&capability_output).context("parse capability detail output")?;
    assert_eq!(capability_json["status"], 200);
    assert_eq!(
        capability_json["body"]["capability"]["capability"],
        "@frontend.dev"
    );

    let resumed_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "events",
        "--session-id",
        "sess_events",
        "--last-event-id",
        "30",
        "--limit",
        "1",
    ])?;
    let resumed_json: Value =
        serde_json::from_str(&resumed_output).context("parse resumed events output")?;
    assert_eq!(resumed_json["status"], 200);
    assert_eq!(resumed_json["body"][0]["event"], "capabilities.updated");
    assert_eq!(
        resumed_json["body"][0]["data"]["data"]["capability"],
        "@frontend.dev"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn broker_client_fixture_combines_leased_mutation_detail_and_event_resume() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_combined_events");
                    assert_eq!(body["client_id"], "fixture-events");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_combined_events",
                                "attachment": {
                                    "client_id": "fixture-events",
                                    "lease_seconds": 45
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_combined_events/events");
                    let event_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 60\n",
                        "event: service.updated\n",
                        "data: {\"session_id\":\"sess_combined_events\",\"job_id\":\"bg-1\",\"label\":\"frontend\",\"capabilities\":[\"@frontend.pending\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(event_stream.as_bytes())
                        .context("write initial event stream")?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_combined_events/services/bg-1/provide"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse provide body")?;
                    assert_eq!(body["client_id"], "fixture-events");
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
                                "label": "frontend",
                                "capabilities": ["@frontend.dev"]
                            },
                            "operation": {
                                "kind": "service.provide"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_combined_events/services/bg-1"
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
                                "label": "frontend",
                                "capabilities": ["@frontend.dev"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_combined_events/events");
                    assert_eq!(
                        request._headers.get("last-event-id").map(String::as_str),
                        Some("60")
                    );
                    let resumed_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 61\n",
                        "event: capabilities.updated\n",
                        "data: {\"session_id\":\"sess_combined_events\",\"capability\":\"@frontend.dev\",\"status\":\"healthy\",\"providers\":[\"bg-1\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(resumed_stream.as_bytes())
                        .context("write resumed event stream")?;
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
        "fixture-events",
        "--lease-seconds",
        "45",
        "session-create",
        "--thread-id",
        "thread_combined_events",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_combined_events"
    );

    let events_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "events",
        "--session-id",
        "sess_combined_events",
        "--limit",
        "1",
    ])?;
    let events_json: Value = serde_json::from_str(&events_output).context("parse events output")?;
    assert_eq!(events_json["status"], 200);
    assert_eq!(events_json["body"][0]["event"], "service.updated");

    let provide_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-events",
        "service-provide",
        "--session-id",
        "sess_combined_events",
        "--job-ref",
        "bg-1",
        "--values-json",
        "[\"@frontend.dev\"]",
    ])?;
    let provide_json: Value =
        serde_json::from_str(&provide_output).context("parse provide output")?;
    assert_eq!(provide_json["status"], 200);
    assert_eq!(provide_json["body"]["operation"]["kind"], "service.provide");

    let detail_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "service-detail",
        "--session-id",
        "sess_combined_events",
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
        "events",
        "--session-id",
        "sess_combined_events",
        "--last-event-id",
        "60",
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
