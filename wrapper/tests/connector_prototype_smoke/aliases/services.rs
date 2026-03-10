use super::*;

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
