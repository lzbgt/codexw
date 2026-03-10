use super::*;

#[test]
fn connector_alias_session_create_rejects_invalid_lease_header_with_structured_error() -> Result<()>
{
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let body = "{\"thread_id\":\"thread_1\"}";
    let request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "X-Codexw-Client-Id: remote-web\r\n",
            "X-Codexw-Lease-Seconds: not-a-number\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    let response = send_raw_request(connector_port, &request)?;
    assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert!(response.contains("\"code\":\"validation_error\""));
    assert!(response.contains("x-codexw-lease-seconds must be a positive integer header"));
    assert!(response.contains("\"field\":\"x-codexw-lease-seconds\""));
    assert!(response.contains("\"expected\":\"positive integer header\""));
    Ok(())
}

#[test]
fn connector_alias_session_create_rejects_non_object_json_for_injection() -> Result<()> {
    let local_api_port = reserve_port()?;
    let connector_port = reserve_port()?;
    let mut connector = spawn_connector(connector_port, local_api_port)?;
    wait_for_healthz(&mut connector, connector_port)?;

    let body = "[\"bad\"]";
    let request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions HTTP/1.1\r\n",
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
    assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert!(response.contains("\"code\":\"validation_error\""));
    assert!(response.contains("connector JSON injection requires a JSON object body"));
    assert!(response.contains("\"field\":\"body\""));
    assert!(response.contains("\"expected\":\"json object\""));
    Ok(())
}

#[test]
fn connector_alias_session_create_preserves_local_field_validation_error() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
        let request = read_http_request(&mut stream)?;
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/session/new");
        let body: Value = serde_json::from_slice(&request.body).context("parse body")?;
        assert_eq!(body["thread_id"], "thread_1");
        assert_eq!(body["client_id"], "");
        assert_eq!(body["lease_seconds"], 0);
        write_http_response(
            &mut stream,
            400,
            "Bad Request",
            &[("Content-Type", "application/json")],
            serde_json::to_vec(&json!({
                "ok": false,
                "error": {
                    "status": 400,
                    "code": "validation_error",
                    "message": "client_id must not be empty",
                    "retryable": false,
                    "details": {
                        "field": "client_id",
                        "expected": "non-empty string"
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

    let body = "{\"thread_id\":\"thread_1\",\"client_id\":\"\",\"lease_seconds\":0}";
    let request = format!(
        concat!(
            "POST /v1/agents/codexw-lab/sessions HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Content-Type: application/json\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        body.len(),
        body
    );
    let response = send_raw_request(connector_port, &request)?;
    assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert!(response.contains("\"code\":\"validation_error\""));
    assert!(response.contains("client_id must not be empty"));
    assert!(response.contains("\"field\":\"client_id\""));
    assert!(response.contains("\"expected\":\"non-empty string\""));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
