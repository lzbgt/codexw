use super::*;

#[test]
fn connector_alias_attach_projects_session_and_lease_headers() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/session/attach");

        let body: Value = serde_json::from_slice(&request.body).context("parse forwarded body")?;
        assert_eq!(body["session_id"], "sess_1");
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
        Ok(())
    });

    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let body = "{\"thread_id\":\"thread_1\"}";
    let request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/attach HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 45\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    let response = send_raw_request(connector_port, &request)?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("\"session_id\":\"sess_1\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_session_create_and_attachment_lifecycle_routes_work() -> Result<()> {
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
                                    "lease_seconds": 45,
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/attachment/renew");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse renew body")?;
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 90);
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
                                    "lease_seconds": 90,
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/attachment/release");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse release body")?;
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_1",
                                "attachment": Value::Null,
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

    let create_body = "{\"thread_id\":\"thread_1\"}";
    let create_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 45\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        create_body.len(),
        create_body
    );
    let create_response = send_raw_request(connector_port, &create_request)?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(create_response.contains("\"session_id\":\"sess_1\""));

    let renew_body = "{\"client_id\":\"remote-web\",\"lease_seconds\":90}";
    let renew_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/attachment/renew HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        renew_body.len(),
        renew_body
    );
    let renew_response = send_raw_request(connector_port, &renew_request)?;
    assert!(renew_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(renew_response.contains("\"lease_seconds\":90"));

    let release_body = "{\"client_id\":\"remote-web\"}";
    let release_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/attachment/release HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        release_body.len(),
        release_body
    );
    let release_response = send_raw_request(connector_port, &release_request)?;
    assert!(release_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(release_response.contains("\"attachment\":null"));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_events_route_wraps_broker_metadata() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/v1/session/sess_1/events");

        let body = concat!(
            ": heartbeat\n",
            "id: 1\n",
            "event: session.updated\n",
            "data: {\"session_id\":\"sess_1\"}\n\n"
        );
        write_http_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "text/event-stream")],
            body.as_bytes(),
        )?;
        Ok(())
    });

    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/events HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("Content-Type: text/event-stream\r\n"));
    assert!(response.contains(": heartbeat\n"));
    assert!(response.contains("id: 1\n"));
    assert!(response.contains("event: session.updated\n"));
    assert!(response.contains("\"source\":\"codexw\""));
    assert!(response.contains("\"agent_id\":\"codexw-lab\""));
    assert!(response.contains("\"deployment_id\":\"mac-mini-01\""));
    assert!(response.contains("\"data\":{\"session_id\":\"sess_1\"}"));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_events_route_forwards_last_event_id() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/v1/session/sess_1/events");
        let request_text = String::from_utf8_lossy(&request.body);
        assert!(request_text.is_empty());

        write_http_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "text/event-stream")],
            concat!(
                "id: 11\n",
                "event: capabilities.updated\n",
                "data: {\"capability\":\"@frontend.dev\"}\n\n"
            )
            .as_bytes(),
        )?;
        Ok(())
    });

    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/events HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Last-Event-ID: 10\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("event: capabilities.updated\n"));
    assert!(response.contains("\"capability\":\"@frontend.dev\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_returns_not_found_for_unknown_broker_alias_route() -> Result<()> {
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/orchestration/plans HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
    assert!(response.contains("\"code\":\"not_found\""));
    assert!(response.contains("unknown connector route"));

    Ok(())
}

#[test]
fn connector_rejects_raw_proxy_route_outside_allowed_surface() -> Result<()> {
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/proxy/api/v1/session/sess_1/scene HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 403 Error\r\n"));
    assert!(response.contains("\"code\":\"route_not_allowed\""));
    assert!(
        response
            .contains("\"message\":\"connector route is outside the allowed local API surface\"")
    );
    assert!(response.contains("\"local_path\":\"/api/v1/session/sess_1/scene\""));
    assert!(response.contains("\"is_sse\":false"));

    Ok(())
}

#[test]
fn connector_rejects_raw_proxy_sse_route_outside_allowed_surface() -> Result<()> {
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/proxy_sse/api/v1/session/sess_1/transcript HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(response.starts_with("HTTP/1.1 403 Error\r\n"));
    assert!(response.contains("\"code\":\"route_not_allowed\""));
    assert!(response.contains("\"local_path\":\"/api/v1/session/sess_1/transcript\""));
    assert!(response.contains("\"is_sse\":true"));

    Ok(())
}

#[test]
fn connector_alias_shell_start_projects_client_and_lease_headers() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/session/sess_1/shells/start");

        let body: Value = serde_json::from_slice(&request.body).context("parse shell body")?;
        assert_eq!(body["command"], "npm run dev");
        assert_eq!(body["client_id"], "remote-web");
        assert_eq!(body["lease_seconds"], 45);

        write_http_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/json")],
            serde_json::to_vec(&json!({
                "ok": true,
                "interaction": { "kind": "shell_start" },
                "job": { "job_id": "bg-1" }
            }))?
            .as_slice(),
        )?;
        Ok(())
    });

    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let body = "{\"command\":\"npm run dev\"}";
    let request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/shells HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 45\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    let response = send_raw_request(connector_port, &request)?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("\"job_id\":\"bg-1\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_service_run_maps_to_local_service_route() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api/run");

        let body: Value =
            serde_json::from_slice(&request.body).context("parse service run body")?;
        assert_eq!(body["recipe"], "health");
        assert_eq!(body["client_id"], "remote-web");

        write_http_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/json")],
            serde_json::to_vec(&json!({
                "ok": true,
                "interaction": { "kind": "service_run" },
                "recipe": { "name": "health" },
                "result": "healthy"
            }))?
            .as_slice(),
        )?;
        Ok(())
    });

    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let body = "{\"recipe\":\"health\"}";
    let request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions/sess_1/services/dev.api/run HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    let response = send_raw_request(connector_port, &request)?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("\"name\":\"health\""));
    assert!(response.contains("\"result\":\"healthy\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_shell_service_and_capability_detail_routes_map_cleanly() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..3 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/shells/@api.http");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "shell": {
                                "id": "bg-1",
                                "alias": "dev.api",
                                "service_capabilities": ["@api.http"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
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
                                "capabilities": ["@api.http"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
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
                                "status": "healthy"
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

    let shell_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/shells/%40api.http HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(shell_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(shell_response.contains("\"alias\":\"dev.api\""));
    assert!(shell_response.contains("\"service_capabilities\":[\"@api.http\"]"));

    let service_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/services/dev.api HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(service_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(service_response.contains("\"alias\":\"dev.api\""));
    assert!(service_response.contains("\"@api.http\""));

    let capability_response = send_raw_request(
        connector_port,
        concat!(
            "GET /v1/agents/codexw-lab/sessions/sess_1/capabilities/%40frontend.dev HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
    )?;
    assert!(capability_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(capability_response.contains("\"name\":\"@frontend.dev\""));
    assert!(capability_response.contains("\"status\":\"healthy\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_alias_turn_and_service_routes_propagate_attachment_conflict() -> Result<()> {
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
                    assert_eq!(body["client_id"], "owner-web");
                    assert_eq!(body["lease_seconds"], 90);
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
                                    "client_id": "owner-web",
                                    "lease_seconds": 90,
                                    "lease_active": true
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
                    assert_eq!(body["prompt"], "conflicting turn");
                    assert_eq!(body["client_id"], "other-web");
                    write_http_response(
                        &mut stream,
                        409,
                        "Conflict",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": false,
                            "error": {
                                "status": 409,
                                "code": "attachment_conflict",
                                "message": "attachment is leased by another client",
                                "retryable": false,
                                "details": {
                                    "requested_client_id": "other-web",
                                    "current_attachment": {
                                        "client_id": "owner-web",
                                        "lease_seconds": 90,
                                        "lease_active": true,
                                    }
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api/run");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse service run body")?;
                    assert_eq!(body["recipe"], "health");
                    assert_eq!(body["client_id"], "other-web");
                    write_http_response(
                        &mut stream,
                        409,
                        "Conflict",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": false,
                            "error": {
                                "status": 409,
                                "code": "attachment_conflict",
                                "message": "attachment is leased by another client",
                                "retryable": false,
                                "details": {
                                    "requested_client_id": "other-web",
                                    "current_attachment": {
                                        "client_id": "owner-web",
                                        "lease_seconds": 90,
                                        "lease_active": true,
                                    }
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

    let client = BrokerClient::new(connector_port, "codexw-lab");
    let create_response = client.create_session(
        "{\"thread_id\":\"thread_1\"}",
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "owner-web"),
            ("X-Codexw-Lease-Seconds", "90"),
        ],
    )?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));

    let turn_response = client.session_request(
        "POST",
        "sess_1",
        "/turns",
        Some("{\"prompt\":\"conflicting turn\"}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "other-web"),
        ],
    )?;
    assert!(turn_response.starts_with("HTTP/1.1 409 Conflict\r\n"));
    assert!(turn_response.contains("\"code\":\"attachment_conflict\""));
    assert!(turn_response.contains("\"requested_client_id\":\"other-web\""));
    assert!(turn_response.contains("\"client_id\":\"owner-web\""));
    assert!(turn_response.contains("\"lease_seconds\":90"));
    assert!(turn_response.contains("\"lease_active\":true"));

    let run_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/run",
        Some("{\"recipe\":\"health\"}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "other-web"),
        ],
    )?;
    assert!(run_response.starts_with("HTTP/1.1 409 Conflict\r\n"));
    assert!(run_response.contains("\"code\":\"attachment_conflict\""));
    assert!(run_response.contains("\"requested_client_id\":\"other-web\""));
    assert!(run_response.contains("\"client_id\":\"owner-web\""));
    assert!(run_response.contains("\"lease_seconds\":90"));
    assert!(run_response.contains("\"lease_active\":true"));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
