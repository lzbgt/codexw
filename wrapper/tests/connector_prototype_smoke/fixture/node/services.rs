use super::*;

#[test]
fn node_broker_client_fixture_drives_connector_service_workflow() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..7 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_node_service");
                    assert_eq!(body["client_id"], "node-service");
                    assert_eq!(body["lease_seconds"], 60);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_node_service"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_node_service/shells/start"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse shell-start body")?;
                    assert_eq!(body["command"], "npm run dev");
                    assert_eq!(body["intent"], "service");
                    assert_eq!(body["label"], "frontend");
                    assert_eq!(body["client_id"], "node-service");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "job": {
                                "job_id": "bg-1",
                                "label": "frontend"
                            },
                            "interaction": {
                                "kind": "shell_start",
                                "job_ref": "bg-1"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_node_service/services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "services": [
                                {
                                    "job_id": "bg-1",
                                    "label": "frontend",
                                    "ready_state": "ready"
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_node_service/services/bg-1/attach"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse attach body")?;
                    assert_eq!(body["client_id"], "node-service");
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
                                "ready_state": "ready"
                            },
                            "interaction": {
                                "kind": "attach",
                                "job_ref": "bg-1"
                            },
                            "attachment_text": "Open http://127.0.0.1:3000"
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_node_service/services/bg-1/wait"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse wait body")?;
                    assert_eq!(body["timeout_ms"], 5000);
                    assert_eq!(body["client_id"], "node-service");
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
                                "ready_state": "ready"
                            },
                            "interaction": {
                                "kind": "wait",
                                "job_ref": "bg-1"
                            },
                            "ready": true
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_node_service/services/bg-1/run"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse run body")?;
                    assert_eq!(body["recipe"], "health");
                    assert_eq!(body["client_id"], "node-service");
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
                                "ready_state": "ready"
                            },
                            "interaction": {
                                "kind": "run",
                                "job_ref": "bg-1"
                            },
                            "recipe": {
                                "name": "health"
                            },
                            "result_text": "healthy"
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_node_service/capabilities"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "capabilities": [
                                {
                                    "capability": "@frontend.dev",
                                    "status": "healthy",
                                    "providers": [
                                        { "job_id": "bg-1" }
                                    ]
                                }
                            ]
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
    let create_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-service",
        "--lease-seconds",
        "60",
        "session-create",
        "--thread-id",
        "thread_node_service",
    ])?;
    let create_json: Value =
        serde_json::from_str(&create_output).context("parse node create output")?;
    assert_eq!(create_json["status"], 200);

    let shell_start_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-service",
        "shell-start",
        "--session-id",
        "sess_node_service",
        "--shell-command",
        "npm run dev",
        "--intent",
        "service",
        "--label",
        "frontend",
    ])?;
    let shell_start_json: Value =
        serde_json::from_str(&shell_start_output).context("parse node shell start output")?;
    assert_eq!(shell_start_json["status"], 200);
    assert_eq!(shell_start_json["body"]["job"]["job_id"], "bg-1");

    let services_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "services",
        "--session-id",
        "sess_node_service",
    ])?;
    let services_json: Value =
        serde_json::from_str(&services_output).context("parse node services output")?;
    assert_eq!(
        services_json["status"], 200,
        "node services output: {services_output}"
    );
    assert_eq!(services_json["body"]["services"][0]["label"], "frontend");

    let attach_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-service",
        "service-attach",
        "--session-id",
        "sess_node_service",
        "--job-ref",
        "bg-1",
    ])?;
    let attach_json: Value =
        serde_json::from_str(&attach_output).context("parse node attach output")?;
    assert_eq!(attach_json["status"], 200);
    assert_eq!(attach_json["body"]["interaction"]["kind"], "attach");

    let wait_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-service",
        "service-wait",
        "--session-id",
        "sess_node_service",
        "--job-ref",
        "bg-1",
        "--timeout-ms",
        "5000",
    ])?;
    let wait_json: Value = serde_json::from_str(&wait_output).context("parse node wait output")?;
    assert_eq!(wait_json["status"], 200);
    assert_eq!(wait_json["body"]["ready"], true);

    let run_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-service",
        "service-run",
        "--session-id",
        "sess_node_service",
        "--job-ref",
        "bg-1",
        "--recipe",
        "health",
    ])?;
    let run_json: Value = serde_json::from_str(&run_output).context("parse node run output")?;
    assert_eq!(run_json["status"], 200);
    assert_eq!(run_json["body"]["recipe"]["name"], "health");
    assert_eq!(run_json["body"]["result_text"], "healthy");

    let capabilities_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "capabilities",
        "--session-id",
        "sess_node_service",
    ])?;
    let capabilities_json: Value =
        serde_json::from_str(&capabilities_output).context("parse node capabilities output")?;
    assert_eq!(capabilities_json["status"], 200);
    assert_eq!(
        capabilities_json["body"]["capabilities"][0]["capability"],
        "@frontend.dev"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn node_broker_client_fixture_reports_attachment_conflict_details() -> Result<()> {
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
                    assert_eq!(body["client_id"], "node-owner");
                    assert_eq!(body["lease_seconds"], 90);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_node_conflict",
                                "attachment": {
                                    "client_id": "node-owner",
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
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_node_conflict/services/bg-1/run"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse run body")?;
                    assert_eq!(body["client_id"], "node-rival");
                    assert_eq!(body["recipe"], "health");
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
                                    "requested_client_id": "node-rival",
                                    "current_attachment": {
                                        "client_id": "node-owner",
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
    let create_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-owner",
        "--lease-seconds",
        "90",
        "session-create",
        "--thread-id",
        "thread_node_conflict",
    ])?;
    let create_json: Value =
        serde_json::from_str(&create_output).context("parse node create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_node_conflict"
    );

    let run_output = run_node_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "node-rival",
        "service-run",
        "--session-id",
        "sess_node_conflict",
        "--job-ref",
        "bg-1",
        "--recipe",
        "health",
    ])?;
    let run_json: Value =
        serde_json::from_str(&run_output).context("parse node conflict output")?;
    assert_eq!(run_json["status"], 409);
    assert_eq!(run_json["body"]["error"]["code"], "attachment_conflict");
    assert_eq!(
        run_json["body"]["error"]["details"]["requested_client_id"],
        "node-rival"
    );
    assert_eq!(
        run_json["body"]["error"]["details"]["current_attachment"]["client_id"],
        "node-owner"
    );
    assert_eq!(
        run_json["body"]["error"]["details"]["current_attachment"]["lease_active"],
        true
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
