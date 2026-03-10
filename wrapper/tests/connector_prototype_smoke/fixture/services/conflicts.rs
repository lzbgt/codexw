use super::*;

#[test]
fn broker_client_fixture_reports_attachment_conflict_details() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..2 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["client_id"], "lease-owner");
                    assert_eq!(body["lease_seconds"], 90);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_conflict",
                                "attachment": {
                                    "client_id": "lease-owner",
                                    "lease_seconds": 90,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 1234567890u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_conflict/turn/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse turn body")?;
                    assert_eq!(body["client_id"], "conflict-client");
                    write_http_response(
                        &mut stream,
                        409,
                        "Conflict",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "status": 409,
                            "error": {
                                "code": "attachment_conflict",
                                "message": "active attachment lease blocks this mutation",
                                "retryable": true,
                                "details": {
                                    "requested_client_id": "conflict-client",
                                    "current_attachment": {
                                        "client_id": "lease-owner",
                                        "lease_seconds": 90,
                                        "lease_expires_at_ms": 1234567890u64,
                                        "lease_active": true
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

    let base_url = format!("http://127.0.0.1:{connector_port}");
    let create_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "lease-owner",
        "--lease-seconds",
        "90",
        "session-create",
        "--thread-id",
        "thread_conflict",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_conflict"
    );

    let turn_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "conflict-client",
        "turn-start",
        "--session-id",
        "sess_conflict",
        "--prompt",
        "This should conflict",
    ])?;
    let turn_json: Value = serde_json::from_str(&turn_output).context("parse turn output")?;
    assert_eq!(turn_json["status"], 409);
    assert_eq!(turn_json["body"]["error"]["code"], "attachment_conflict");
    assert_eq!(
        turn_json["body"]["error"]["details"]["requested_client_id"],
        "conflict-client"
    );
    assert_eq!(
        turn_json["body"]["error"]["details"]["current_attachment"]["client_id"],
        "lease-owner"
    );
    assert_eq!(
        turn_json["body"]["error"]["details"]["current_attachment"]["lease_active"],
        true
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
