use super::*;

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
