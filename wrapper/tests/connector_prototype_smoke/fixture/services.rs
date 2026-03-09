use super::super::*;

#[test]
fn broker_client_fixture_drives_connector_shell_service_workflow() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_shell");
                    assert_eq!(body["client_id"], "fixture-shell");
                    assert_eq!(body["lease_seconds"], 60);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_shell",
                                "attachment": {
                                    "client_id": "fixture-shell",
                                    "lease_seconds": 60
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_shell/shells/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse shell start body")?;
                    assert_eq!(body["command"], "npm run dev");
                    assert_eq!(body["label"], "frontend");
                    assert_eq!(body["client_id"], "fixture-shell");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "job": {
                                "job_id": "bg-1",
                                "label": "frontend",
                                "intent": "service"
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
                    assert_eq!(request.path, "/api/v1/session/sess_shell/services");
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
                        "/api/v1/session/sess_shell/services/bg-1/attach"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse attach body")?;
                    assert_eq!(body["client_id"], "fixture-shell");
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
                        "/api/v1/session/sess_shell/services/bg-1/wait"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse wait body")?;
                    assert_eq!(body["timeout_ms"], 5000);
                    assert_eq!(body["client_id"], "fixture-shell");
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
                                "job_ref": "bg-1",
                                "timeout_ms": 5000
                            },
                            "ready": true
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_shell/services/bg-1/run");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse run body")?;
                    assert_eq!(body["recipe"], "health");
                    assert_eq!(body["client_id"], "fixture-shell");
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
        "fixture-shell",
        "--lease-seconds",
        "60",
        "session-create",
        "--thread-id",
        "thread_shell",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(create_json["body"]["session"]["session_id"], "sess_shell");

    let shell_start_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-shell",
        "shell-start",
        "--session-id",
        "sess_shell",
        "--shell-command",
        "npm run dev",
        "--intent",
        "service",
        "--label",
        "frontend",
    ])?;
    let shell_start_json: Value =
        serde_json::from_str(&shell_start_output).context("parse shell start output")?;
    assert_eq!(shell_start_json["status"], 200);
    assert_eq!(shell_start_json["body"]["job"]["job_id"], "bg-1");

    let services_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "services",
        "--session-id",
        "sess_shell",
    ])?;
    let services_json: Value =
        serde_json::from_str(&services_output).context("parse services output")?;
    assert_eq!(services_json["status"], 200);
    assert_eq!(services_json["body"]["services"][0]["label"], "frontend");

    let attach_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-shell",
        "service-attach",
        "--session-id",
        "sess_shell",
        "--job-ref",
        "bg-1",
    ])?;
    let attach_json: Value = serde_json::from_str(&attach_output).context("parse attach output")?;
    assert_eq!(attach_json["status"], 200);
    assert_eq!(attach_json["body"]["interaction"]["kind"], "attach");

    let wait_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-shell",
        "service-wait",
        "--session-id",
        "sess_shell",
        "--job-ref",
        "bg-1",
        "--timeout-ms",
        "5000",
    ])?;
    let wait_json: Value = serde_json::from_str(&wait_output).context("parse wait output")?;
    assert_eq!(wait_json["status"], 200);
    assert_eq!(wait_json["body"]["ready"], true);

    let run_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-shell",
        "service-run",
        "--session-id",
        "sess_shell",
        "--job-ref",
        "bg-1",
        "--recipe",
        "health",
    ])?;
    let run_json: Value = serde_json::from_str(&run_output).context("parse run output")?;
    assert_eq!(run_json["status"], 200);
    assert_eq!(run_json["body"]["recipe"]["name"], "health");
    assert_eq!(run_json["body"]["result_text"], "healthy");

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn broker_client_fixture_drives_connector_service_mutation_workflow() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_mutations");
                    assert_eq!(body["client_id"], "fixture-mutations");
                    assert_eq!(body["lease_seconds"], 75);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_mutations",
                                "attachment": {
                                    "client_id": "fixture-mutations",
                                    "lease_seconds": 75
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
                        "/api/v1/session/sess_mutations/services/bg-1/provide"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse provide body")?;
                    assert_eq!(body["client_id"], "fixture-mutations");
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
                            "interaction": {
                                "kind": "provide",
                                "job_ref": "bg-1"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_mutations/services/bg-1/depend"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse depend body")?;
                    assert_eq!(body["client_id"], "fixture-mutations");
                    assert_eq!(body["dependsOnCapabilities"], json!(["@api.http"]));
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "depends_on_capabilities": ["@api.http"]
                            },
                            "interaction": {
                                "kind": "depend",
                                "job_ref": "bg-1"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_mutations/services/bg-1/contract"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse contract body")?;
                    assert_eq!(body["client_id"], "fixture-mutations");
                    assert_eq!(body["protocol"], "http");
                    assert_eq!(body["endpoint"], "http://127.0.0.1:3000");
                    assert_eq!(body["attachHint"], "Open UI");
                    assert_eq!(body["readyPattern"], "ready");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "protocol": "http",
                                "endpoint": "http://127.0.0.1:3000",
                                "attach_hint": "Open UI",
                                "ready_pattern": "ready"
                            },
                            "interaction": {
                                "kind": "contract",
                                "job_ref": "bg-1"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_mutations/services/bg-1/relabel"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse relabel body")?;
                    assert_eq!(body["client_id"], "fixture-mutations");
                    assert_eq!(body["label"], "frontend-dev");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "label": "frontend-dev"
                            },
                            "interaction": {
                                "kind": "relabel",
                                "job_ref": "bg-1"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_mutations/services/bg-1");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "label": "frontend-dev",
                                "capabilities": ["@frontend.dev"],
                                "depends_on_capabilities": ["@api.http"],
                                "protocol": "http",
                                "endpoint": "http://127.0.0.1:3000",
                                "attach_hint": "Open UI",
                                "ready_pattern": "ready"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_mutations/capabilities/@frontend.dev"
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
                                "providers": ["bg-1"],
                                "consumers": []
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
        "fixture-mutations",
        "--lease-seconds",
        "75",
        "session-create",
        "--thread-id",
        "thread_mutations",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_mutations"
    );

    let provide_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-mutations",
        "service-provide",
        "--session-id",
        "sess_mutations",
        "--job-ref",
        "bg-1",
        "--values-json",
        "[\"@frontend.dev\"]",
    ])?;
    let provide_json: Value =
        serde_json::from_str(&provide_output).context("parse provide output")?;
    assert_eq!(provide_json["status"], 200);
    assert_eq!(provide_json["body"]["interaction"]["kind"], "provide");

    let depend_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-mutations",
        "service-depend",
        "--session-id",
        "sess_mutations",
        "--job-ref",
        "bg-1",
        "--values-json",
        "[\"@api.http\"]",
    ])?;
    let depend_json: Value = serde_json::from_str(&depend_output).context("parse depend output")?;
    assert_eq!(depend_json["status"], 200);
    assert_eq!(depend_json["body"]["interaction"]["kind"], "depend");

    let contract_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-mutations",
        "service-contract",
        "--session-id",
        "sess_mutations",
        "--job-ref",
        "bg-1",
        "--contract-json",
        "{\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\",\"attachHint\":\"Open UI\",\"readyPattern\":\"ready\"}",
    ])?;
    let contract_json: Value =
        serde_json::from_str(&contract_output).context("parse contract output")?;
    assert_eq!(contract_json["status"], 200);
    assert_eq!(contract_json["body"]["interaction"]["kind"], "contract");

    let relabel_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-mutations",
        "service-relabel",
        "--session-id",
        "sess_mutations",
        "--job-ref",
        "bg-1",
        "--label",
        "frontend-dev",
    ])?;
    let relabel_json: Value =
        serde_json::from_str(&relabel_output).context("parse relabel output")?;
    assert_eq!(relabel_json["status"], 200);
    assert_eq!(relabel_json["body"]["interaction"]["kind"], "relabel");

    let service_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "service-detail",
        "--session-id",
        "sess_mutations",
        "--job-ref",
        "bg-1",
    ])?;
    let service_json: Value =
        serde_json::from_str(&service_output).context("parse service detail output")?;
    assert_eq!(service_json["status"], 200);
    assert_eq!(service_json["body"]["service"]["label"], "frontend-dev");
    assert_eq!(
        service_json["body"]["service"]["capabilities"],
        json!(["@frontend.dev"])
    );
    assert_eq!(
        service_json["body"]["service"]["depends_on_capabilities"],
        json!(["@api.http"])
    );

    let capability_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "capability-detail",
        "--session-id",
        "sess_mutations",
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
    assert_eq!(
        capability_json["body"]["capability"]["providers"],
        json!(["bg-1"])
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

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
