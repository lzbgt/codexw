use super::*;

#[test]
fn broker_client_fixture_preserves_observer_reads_during_lease_contention() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..8 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_observer_reads");
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["lease_seconds"], 120);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_observer_reads",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 120,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 5233445566u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_observer_reads");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_observer_reads",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 120,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 5233445566u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_observer_reads/orchestration/status"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session_id": "sess_observer_reads",
                            "main_agent": {
                                "state": "runnable"
                            },
                            "next_action": "observe current orchestration state"
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_observer_reads/shells/bg-1"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "job": {
                                "job_id": "bg-1",
                                "alias": "dev.build",
                                "intent": "observation",
                                "status": "running"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_observer_reads/services/bg-2"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-2",
                                "alias": "dev.api",
                                "capabilities": ["@frontend.pending"],
                                "ready_state": "booting"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_observer_reads/capabilities/@frontend.pending"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "capability": {
                                "name": "@frontend.pending",
                                "status": "booting",
                                "providers": [{
                                    "job_id": "bg-2",
                                    "alias": "dev.api"
                                }]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_observer_reads/services/bg-2/provide"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse rival provide body")?;
                    assert_eq!(body["client_id"], "fixture-rival");
                    assert_eq!(body["capabilities"], json!(["@frontend.live"]));
                    write_http_response(
                        &mut stream,
                        409,
                        "Conflict",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": false,
                            "error": {
                                "status": 409,
                                "code": "attachment_conflict",
                                "message": "active attachment lease blocks this mutation",
                                "retryable": false,
                                "details": {
                                    "requested_client_id": "fixture-rival",
                                    "current_attachment": {
                                        "client_id": "fixture-owner",
                                        "lease_seconds": 120,
                                        "lease_expires_at_ms": 5233445566u64,
                                        "lease_active": true
                                    }
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                7 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_observer_reads/services/bg-2"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-2",
                                "alias": "dev.api",
                                "capabilities": ["@frontend.pending"],
                                "ready_state": "booting"
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
        "fixture-owner",
        "--lease-seconds",
        "120",
        "session-create",
        "--thread-id",
        "thread_observer_reads",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_observer_reads"
    );

    let session_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "session-get",
        "--session-id",
        "sess_observer_reads",
    ])?;
    let session_json: Value =
        serde_json::from_str(&session_output).context("parse session output")?;
    assert_eq!(session_json["status"], 200);
    assert_eq!(
        session_json["body"]["session"]["attachment"]["client_id"],
        "fixture-owner"
    );

    let orchestration_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "orchestration-status",
        "--session-id",
        "sess_observer_reads",
    ])?;
    let orchestration_json: Value =
        serde_json::from_str(&orchestration_output).context("parse orchestration output")?;
    assert_eq!(orchestration_json["status"], 200);
    assert_eq!(
        orchestration_json["body"]["main_agent"]["state"],
        "runnable"
    );

    let shell_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "shell-detail",
        "--session-id",
        "sess_observer_reads",
        "--job-ref",
        "bg-1",
    ])?;
    let shell_json: Value = serde_json::from_str(&shell_output).context("parse shell output")?;
    assert_eq!(shell_json["status"], 200);
    assert_eq!(shell_json["body"]["job"]["alias"], "dev.build");

    let service_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "service-detail",
        "--session-id",
        "sess_observer_reads",
        "--job-ref",
        "bg-2",
    ])?;
    let service_json: Value =
        serde_json::from_str(&service_output).context("parse service output")?;
    assert_eq!(service_json["status"], 200);
    assert_eq!(service_json["body"]["service"]["alias"], "dev.api");

    let capability_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "capability-detail",
        "--session-id",
        "sess_observer_reads",
        "--capability",
        "@frontend.pending",
    ])?;
    let capability_json: Value =
        serde_json::from_str(&capability_output).context("parse capability output")?;
    assert_eq!(capability_json["status"], 200);
    assert_eq!(
        capability_json["body"]["capability"]["name"],
        "@frontend.pending"
    );

    let rival_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-rival",
        "service-provide",
        "--session-id",
        "sess_observer_reads",
        "--job-ref",
        "bg-2",
        "--values-json",
        "[\"@frontend.live\"]",
    ])?;
    let rival_json: Value = serde_json::from_str(&rival_output).context("parse rival output")?;
    assert_eq!(rival_json["status"], 409);
    assert_eq!(rival_json["body"]["error"]["code"], "attachment_conflict");

    let service_after_conflict_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-observer",
        "service-detail",
        "--session-id",
        "sess_observer_reads",
        "--job-ref",
        "bg-2",
    ])?;
    let service_after_conflict_json: Value = serde_json::from_str(&service_after_conflict_output)
        .context("parse service-after-conflict output")?;
    assert_eq!(service_after_conflict_json["status"], 200);
    assert_eq!(
        service_after_conflict_json["body"]["service"]["capabilities"][0],
        "@frontend.pending"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn broker_client_fixture_allows_anonymous_observer_reads_and_blocks_anonymous_rival_mutation()
-> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..8 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/new");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse create body")?;
                    assert_eq!(body["thread_id"], "thread_anonymous_observer_reads");
                    assert_eq!(body["client_id"], "fixture-owner");
                    assert_eq!(body["lease_seconds"], 120);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_anonymous_observer_reads",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 120,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 6233445566u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_anonymous_observer_reads/events"
                    );
                    let event_stream = concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Type: text/event-stream\r\n",
                        "Connection: close\r\n",
                        "\r\n",
                        ": heartbeat\n",
                        "id: 310\n",
                        "event: service.updated\n",
                        "data: {\"session_id\":\"sess_anonymous_observer_reads\",\"job_id\":\"bg-2\",\"alias\":\"dev.api\",\"capabilities\":[\"@frontend.pending\"]}\n",
                        "\n"
                    );
                    stream
                        .write_all(event_stream.as_bytes())
                        .context("write anonymous observer event stream")?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_anonymous_observer_reads"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_anonymous_observer_reads",
                                "attachment": {
                                    "client_id": "fixture-owner",
                                    "lease_seconds": 120,
                                    "lease_active": true,
                                    "lease_expires_at_ms": 6233445566u64
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_anonymous_observer_reads/orchestration/status"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session_id": "sess_anonymous_observer_reads",
                            "main_agent": {
                                "state": "runnable"
                            },
                            "next_action": "observe anonymous access semantics"
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_anonymous_observer_reads/services/bg-2"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-2",
                                "alias": "dev.api",
                                "capabilities": ["@frontend.pending"],
                                "ready_state": "booting"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_anonymous_observer_reads/capabilities/@frontend.pending"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "capability": {
                                "name": "@frontend.pending",
                                "status": "booting",
                                "providers": [{
                                    "job_id": "bg-2",
                                    "alias": "dev.api"
                                }]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_anonymous_observer_reads/services/bg-2/provide"
                    );
                    let body: Value = serde_json::from_slice(&request.body)
                        .context("parse anonymous rival provide body")?;
                    assert!(body["client_id"].is_null());
                    assert_eq!(body["capabilities"], json!(["@frontend.live"]));
                    write_http_response(
                        &mut stream,
                        409,
                        "Conflict",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": false,
                            "error": {
                                "status": 409,
                                "code": "attachment_conflict",
                                "message": "active attachment lease blocks this mutation",
                                "retryable": false,
                                "details": {
                                    "requested_client_id": Value::Null,
                                    "current_attachment": {
                                        "client_id": "fixture-owner",
                                        "lease_seconds": 120,
                                        "lease_expires_at_ms": 6233445566u64,
                                        "lease_active": true
                                    }
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                7 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_anonymous_observer_reads/services/bg-2"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-2",
                                "alias": "dev.api",
                                "capabilities": ["@frontend.pending"],
                                "ready_state": "booting"
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
        "fixture-owner",
        "--lease-seconds",
        "120",
        "session-create",
        "--thread-id",
        "thread_anonymous_observer_reads",
    ])?;
    let create_json: Value = serde_json::from_str(&create_output).context("parse create output")?;
    assert_eq!(create_json["status"], 200);
    assert_eq!(
        create_json["body"]["session"]["session_id"],
        "sess_anonymous_observer_reads"
    );

    let anonymous_events_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "events",
        "--session-id",
        "sess_anonymous_observer_reads",
        "--limit",
        "1",
    ])?;
    let anonymous_events_json: Value =
        serde_json::from_str(&anonymous_events_output).context("parse anonymous events output")?;
    assert_eq!(anonymous_events_json["status"], 200);
    assert_eq!(anonymous_events_json["body"][0]["event"], "service.updated");

    let session_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "session-get",
        "--session-id",
        "sess_anonymous_observer_reads",
    ])?;
    let session_json: Value =
        serde_json::from_str(&session_output).context("parse anonymous session output")?;
    assert_eq!(session_json["status"], 200);
    assert_eq!(
        session_json["body"]["session"]["attachment"]["client_id"],
        "fixture-owner"
    );

    let orchestration_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "orchestration-status",
        "--session-id",
        "sess_anonymous_observer_reads",
    ])?;
    let orchestration_json: Value = serde_json::from_str(&orchestration_output)
        .context("parse anonymous orchestration output")?;
    assert_eq!(orchestration_json["status"], 200);
    assert_eq!(
        orchestration_json["body"]["main_agent"]["state"],
        "runnable"
    );

    let service_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "service-detail",
        "--session-id",
        "sess_anonymous_observer_reads",
        "--job-ref",
        "bg-2",
    ])?;
    let service_json: Value =
        serde_json::from_str(&service_output).context("parse anonymous service output")?;
    assert_eq!(service_json["status"], 200);
    assert_eq!(service_json["body"]["service"]["alias"], "dev.api");

    let capability_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "capability-detail",
        "--session-id",
        "sess_anonymous_observer_reads",
        "--capability",
        "@frontend.pending",
    ])?;
    let capability_json: Value =
        serde_json::from_str(&capability_output).context("parse anonymous capability output")?;
    assert_eq!(capability_json["status"], 200);
    assert_eq!(
        capability_json["body"]["capability"]["name"],
        "@frontend.pending"
    );

    let anonymous_rival_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "service-provide",
        "--session-id",
        "sess_anonymous_observer_reads",
        "--job-ref",
        "bg-2",
        "--values-json",
        "[\"@frontend.live\"]",
    ])?;
    let anonymous_rival_json: Value =
        serde_json::from_str(&anonymous_rival_output).context("parse anonymous rival output")?;
    assert_eq!(anonymous_rival_json["status"], 409);
    assert_eq!(
        anonymous_rival_json["body"]["error"]["code"],
        "attachment_conflict"
    );
    assert!(anonymous_rival_json["body"]["error"]["details"]["requested_client_id"].is_null());
    assert_eq!(
        anonymous_rival_json["body"]["error"]["details"]["current_attachment"]["client_id"],
        "fixture-owner"
    );

    let service_after_conflict_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "service-detail",
        "--session-id",
        "sess_anonymous_observer_reads",
        "--job-ref",
        "bg-2",
    ])?;
    let service_after_conflict_json: Value =
        serde_json::from_str(&service_after_conflict_output)
            .context("parse anonymous service-after-conflict output")?;
    assert_eq!(service_after_conflict_json["status"], 200);
    assert_eq!(
        service_after_conflict_json["body"]["service"]["capabilities"][0],
        "@frontend.pending"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
