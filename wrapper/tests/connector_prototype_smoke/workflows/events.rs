use super::*;

#[test]
fn connector_broker_style_service_workflow_handles_event_resume() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_1");
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "attachment": {
                                    "client_id": "remote-web",
                                    "lease_seconds": 45
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/events");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "text/event-stream")],
                        concat!(
                            ": heartbeat\n",
                            "id: 10\n",
                            "event: service.updated\n",
                            "data: {\"job_id\":\"bg-1\",\"alias\":\"dev.api\"}\n\n"
                        )
                        .as_bytes(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/attach"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse attach body")?;
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "alias": "dev.api",
                                "ready_state": "ready"
                            },
                            "attachment_text": "curl http://127.0.0.1:8080/health"
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api/wait");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse wait body")?;
                    assert_eq!(body["timeout_ms"], 5000);
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "alias": "dev.api",
                                "ready_state": "ready"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api/run");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse run body")?;
                    assert_eq!(body["recipe"], "health");
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "recipe": { "name": "health" },
                            "result": "healthy"
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/events");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "text/event-stream")],
                        concat!(
                            ": heartbeat\n",
                            "id: 11\n",
                            "event: capabilities.updated\n",
                            "data: {\"capability\":\"@api.http\",\"status\":\"healthy\"}\n\n"
                        )
                        .as_bytes(),
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

    let client = BrokerClient::new(connector_port, "codexw-lab");
    let create_response = client.create_session(
        "{\"thread_id\":\"thread_1\"}",
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
            ("X-Codexw-Lease-Seconds", "45"),
        ],
    )?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));

    let initial_events = client.session_request("GET", "sess_1", "/events", None, &[])?;
    assert!(initial_events.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(initial_events.contains(": heartbeat\n"));
    assert!(initial_events.contains("event: service.updated\n"));
    assert!(initial_events.contains("\"source\":\"codexw\""));
    assert!(initial_events.contains("\"agent_id\":\"codexw-lab\""));
    assert!(initial_events.contains("\"deployment_id\":\"mac-mini-01\""));
    assert!(initial_events.contains("\"job_id\":\"bg-1\""));
    assert!(initial_events.contains("\"alias\":\"dev.api\""));

    let attach_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/attach",
        Some("{}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(attach_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(attach_response.contains("curl http://127.0.0.1:8080/health"));

    let wait_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/wait",
        Some("{\"timeout_ms\":5000}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(wait_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(wait_response.contains("\"ready_state\":\"ready\""));

    let run_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/run",
        Some("{\"recipe\":\"health\"}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(run_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(run_response.contains("\"name\":\"health\""));
    assert!(run_response.contains("\"result\":\"healthy\""));

    let resumed_events =
        client.session_request("GET", "sess_1", "/events", None, &[("Last-Event-ID", "10")])?;
    assert!(resumed_events.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(resumed_events.contains(": heartbeat\n"));
    assert!(resumed_events.contains("event: capabilities.updated\n"));
    assert!(resumed_events.contains("\"source\":\"codexw\""));
    assert!(resumed_events.contains("\"agent_id\":\"codexw-lab\""));
    assert!(resumed_events.contains("\"deployment_id\":\"mac-mini-01\""));
    assert!(resumed_events.contains("\"capability\":\"@api.http\""));
    assert!(resumed_events.contains("\"status\":\"healthy\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_broker_style_focused_detail_workflow_handles_event_resume() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_1");
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "attachment": {
                                    "client_id": "remote-web",
                                    "lease_seconds": 45
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/events");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "text/event-stream")],
                        concat!(
                            ": heartbeat\n",
                            "id: 20\n",
                            "event: service.updated\n",
                            "data: {\"job_id\":\"bg-1\",\"alias\":\"dev.api\",\"capabilities\":[\"@api.http\"]}\n\n"
                        )
                        .as_bytes(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/provide"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse provide body")?;
                    assert_eq!(body["capabilities"], json!(["@api.http", "@frontend.dev"]));
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "alias": "dev.api",
                                "capabilities": ["@api.http", "@frontend.dev"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "alias": "dev.api",
                                "capabilities": ["@api.http", "@frontend.dev"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/capabilities/@frontend.dev"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "capability": {
                                "name": "@frontend.dev",
                                "status": "healthy",
                                "providers": ["bg-1"],
                                "consumers": []
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/events");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "text/event-stream")],
                        concat!(
                            ": heartbeat\n",
                            "id: 21\n",
                            "event: capabilities.updated\n",
                            "data: {\"capability\":\"@frontend.dev\",\"status\":\"healthy\",\"providers\":[\"bg-1\"]}\n\n"
                        )
                        .as_bytes(),
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

    let client = BrokerClient::new(connector_port, "codexw-lab");
    let create_response = client.create_session(
        "{\"thread_id\":\"thread_1\"}",
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
            ("X-Codexw-Lease-Seconds", "45"),
        ],
    )?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));

    let initial_events = client.session_request("GET", "sess_1", "/events", None, &[])?;
    assert!(initial_events.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(initial_events.contains("event: service.updated\n"));
    assert!(initial_events.contains("\"source\":\"codexw\""));
    assert!(initial_events.contains("\"job_id\":\"bg-1\""));
    assert!(initial_events.contains("\"alias\":\"dev.api\""));

    let provide_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/provide",
        Some("{\"capabilities\":[\"@api.http\",\"@frontend.dev\"]}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(provide_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(provide_response.contains("\"@frontend.dev\""));

    let service_response =
        client.session_request("GET", "sess_1", "/services/dev.api", None, &[])?;
    assert!(service_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(service_response.contains("\"alias\":\"dev.api\""));
    assert!(service_response.contains("\"@frontend.dev\""));

    let capability_response =
        client.session_request("GET", "sess_1", "/capabilities/%40frontend.dev", None, &[])?;
    assert!(capability_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(capability_response.contains("\"name\":\"@frontend.dev\""));
    assert!(capability_response.contains("\"providers\":[\"bg-1\"]"));

    let resumed_events =
        client.session_request("GET", "sess_1", "/events", None, &[("Last-Event-ID", "20")])?;
    assert!(resumed_events.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(resumed_events.contains("event: capabilities.updated\n"));
    assert!(resumed_events.contains("\"source\":\"codexw\""));
    assert!(resumed_events.contains("\"capability\":\"@frontend.dev\""));
    assert!(resumed_events.contains("\"providers\":[\"bg-1\"]"));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_broker_style_status_workflow_handles_supervision_event_resume() -> Result<()> {
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
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 45);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "attachment": {
                                    "client_id": "remote-web",
                                    "lease_seconds": 45
                                },
                                "supervision_notice": {
                                    "classification": "tool_slow",
                                    "recommended_action": "observe_or_interrupt",
                                    "recovery_policy": {
                                        "kind": "warn_only",
                                        "automation_ready": false
                                    },
                                    "tool": "background_shell_start",
                                    "summary": "arguments= command=sleep 5 tool=background_shell_start"
                                },
                                "async_tool_supervision": {
                                    "classification": "tool_slow",
                                    "recommended_action": "observe_or_interrupt",
                                    "recovery_policy": {
                                        "kind": "warn_only",
                                        "automation_ready": false
                                    },
                                    "tool": "background_shell_start"
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/events");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "text/event-stream")],
                        concat!(
                            ": heartbeat\n",
                            "id: 30\n",
                            "event: status.updated\n",
                            "data: {\"session_id\":\"sess_1\",\"thread_id\":\"thread_1\",\"turn_running\":true,\"async_tool_supervision\":{\"classification\":\"tool_slow\",\"recommended_action\":\"observe_or_interrupt\",\"recovery_policy\":{\"kind\":\"warn_only\",\"automation_ready\":false},\"tool\":\"background_shell_start\",\"summary\":\"arguments= command=sleep 5 tool=background_shell_start\",\"elapsed_seconds\":21,\"active_request_count\":1},\"supervision_notice\":{\"classification\":\"tool_slow\",\"recommended_action\":\"observe_or_interrupt\",\"recovery_policy\":{\"kind\":\"warn_only\",\"automation_ready\":false},\"tool\":\"background_shell_start\",\"summary\":\"arguments= command=sleep 5 tool=background_shell_start\"}}\n\n"
                        )
                        .as_bytes(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/events");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "text/event-stream")],
                        concat!(
                            ": heartbeat\n",
                            "id: 31\n",
                            "event: status.updated\n",
                            "data: {\"session_id\":\"sess_1\",\"thread_id\":\"thread_1\",\"turn_running\":true,\"async_tool_supervision\":{\"classification\":\"tool_wedged\",\"recommended_action\":\"interrupt_or_exit_resume\",\"recovery_policy\":{\"kind\":\"operator_interrupt_or_exit_resume\",\"automation_ready\":false},\"tool\":\"background_shell_start\",\"summary\":\"arguments= command=sleep 5 tool=background_shell_start\",\"elapsed_seconds\":75,\"active_request_count\":1},\"supervision_notice\":{\"classification\":\"tool_wedged\",\"recommended_action\":\"interrupt_or_exit_resume\",\"recovery_policy\":{\"kind\":\"operator_interrupt_or_exit_resume\",\"automation_ready\":false},\"tool\":\"background_shell_start\",\"summary\":\"arguments= command=sleep 5 tool=background_shell_start\"}}\n\n"
                        )
                        .as_bytes(),
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

    let client = BrokerClient::new(connector_port, "codexw-lab");
    let create_response = client.create_session(
        "{\"thread_id\":\"thread_1\"}",
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
            ("X-Codexw-Lease-Seconds", "45"),
        ],
    )?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(create_response.contains("\"async_tool_supervision\""));
    assert!(create_response.contains("\"supervision_notice\""));

    let initial_events = client.session_request("GET", "sess_1", "/events", None, &[])?;
    assert!(initial_events.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(initial_events.contains(": heartbeat\n"));
    assert!(initial_events.contains("event: status.updated\n"));
    assert!(initial_events.contains("\"source\":\"codexw\""));
    assert!(initial_events.contains("\"agent_id\":\"codexw-lab\""));
    assert!(initial_events.contains("\"deployment_id\":\"mac-mini-01\""));
    assert!(initial_events.contains("\"classification\":\"tool_slow\""));
    assert!(initial_events.contains("\"recommended_action\":\"observe_or_interrupt\""));
    assert!(initial_events.contains("\"kind\":\"warn_only\""));
    assert!(initial_events.contains("\"supervision_notice\""));
    assert!(initial_events.contains("\"tool\":\"background_shell_start\""));

    let resumed_events =
        client.session_request("GET", "sess_1", "/events", None, &[("Last-Event-ID", "30")])?;
    assert!(resumed_events.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(resumed_events.contains(": heartbeat\n"));
    assert!(resumed_events.contains("event: status.updated\n"));
    assert!(resumed_events.contains("\"source\":\"codexw\""));
    assert!(resumed_events.contains("\"agent_id\":\"codexw-lab\""));
    assert!(resumed_events.contains("\"deployment_id\":\"mac-mini-01\""));
    assert!(resumed_events.contains("\"classification\":\"tool_wedged\""));
    assert!(resumed_events.contains("\"recommended_action\":\"interrupt_or_exit_resume\""));
    assert!(resumed_events.contains("\"kind\":\"operator_interrupt_or_exit_resume\""));
    assert!(resumed_events.contains("\"supervision_notice\""));
    assert!(resumed_events.contains("\"elapsed_seconds\":75"));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
