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
fn connector_raw_proxy_turn_routes_work_with_client_header_projection() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..2 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/turn/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse turn start body")?;
                    assert_eq!(body["session_id"], "sess_1");
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 45);
                    assert_eq!(body["input"]["text"], "review this diff");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "accepted": true,
                            "operation": {
                                "kind": "turn.start",
                                "requested_client_id": "remote-web",
                            },
                            "session_id": "sess_1",
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/turn/interrupt");
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse turn interrupt body")?;
                    assert_eq!(body["session_id"], "sess_1");
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "accepted": true,
                            "operation": {
                                "kind": "turn.interrupt",
                                "requested_client_id": "remote-web",
                            },
                            "session_id": "sess_1",
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

    let start_body = "{\"session_id\":\"sess_1\",\"input\":{\"text\":\"review this diff\"}}";
    let start_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/proxy/api/v1/turn/start HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 45\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        start_body.len(),
        start_body
    );
    let start_response = send_raw_request(connector_port, &start_request)?;
    assert!(start_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(start_response.contains("\"kind\":\"turn.start\""));

    let interrupt_body = "{\"session_id\":\"sess_1\"}";
    let interrupt_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/proxy/api/v1/turn/interrupt HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        interrupt_body.len(),
        interrupt_body
    );
    let interrupt_response = send_raw_request(connector_port, &interrupt_request)?;
    assert!(interrupt_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(interrupt_response.contains("\"kind\":\"turn.interrupt\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_raw_proxy_session_and_client_event_routes_work_with_header_projection() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_raw_proxy");
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
                                "session_id": "sess_raw_proxy",
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
                    assert_eq!(request.path, "/api/v1/session/attach");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse attach body")?;
                    assert_eq!(body["session_id"], "sess_raw_proxy");
                    assert_eq!(body["thread_id"], "thread_raw_proxy");
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 60);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_raw_proxy",
                                "attachment": {
                                    "client_id": "remote-web",
                                    "lease_seconds": 60,
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/client_event");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse client event body")?;
                    assert_eq!(body["session_id"], "sess_raw_proxy");
                    assert_eq!(body["client_id"], "remote-web");
                    assert_eq!(body["lease_seconds"], 60);
                    assert_eq!(body["event"], "selection.changed");
                    assert_eq!(body["data"]["selection"], "services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "session_id": "sess_raw_proxy",
                            "client_id": "remote-web",
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
                _ => unreachable!(),
            }
        }
        Ok(())
    });

    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_addr.port())?;
    wait_for_healthz(&mut connector, connector_port)?;

    let create_body = "{\"thread_id\":\"thread_raw_proxy\"}";
    let create_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/proxy/api/v1/session/new HTTP/1.1\r\n",
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
    assert!(create_response.contains("\"session_id\":\"sess_raw_proxy\""));

    let attach_body = "{\"session_id\":\"sess_raw_proxy\",\"thread_id\":\"thread_raw_proxy\"}";
    let attach_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/proxy/api/v1/session/attach HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 60\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        attach_body.len(),
        attach_body
    );
    let attach_response = send_raw_request(connector_port, &attach_request)?;
    assert!(attach_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(attach_response.contains("\"lease_seconds\":60"));

    let client_event_body = concat!(
        "{\"session_id\":\"sess_raw_proxy\",",
        "\"event\":\"selection.changed\",",
        "\"data\":{\"selection\":\"services\"}}"
    );
    let client_event_request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/proxy/api/v1/session/client_event HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: 60\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        client_event_body.len(),
        client_event_body
    );
    let client_event_response = send_raw_request(connector_port, &client_event_request)?;
    assert!(client_event_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(client_event_response.contains("\"kind\":\"client.event\""));
    assert!(client_event_response.contains("\"selection\":\"services\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
