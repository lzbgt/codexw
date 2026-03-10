use super::*;

#[test]
fn broker_client_fixture_drives_connector_attach_and_orchestration_workflow() -> Result<()> {
    let local_listener = TcpListener::bind("127.0.0.1:0").context("bind fake local api")?;
    let local_addr = local_listener.local_addr().context("local api addr")?;

    let fake_server = thread::spawn(move || -> Result<()> {
        for expected in 0..4 {
            let (mut stream, _) = local_listener.accept().context("accept fake local api")?;
            let request = read_http_request(&mut stream)?;
            match expected {
                0 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/attach");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse attach body")?;
                    assert_eq!(body["session_id"], "sess_attach_existing");
                    assert_eq!(body["thread_id"], "thread_existing");
                    assert_eq!(body["client_id"], "fixture-attach");
                    assert_eq!(body["lease_seconds"], 55);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_attach_existing",
                                "thread_id": "thread_existing",
                                "attachment": {
                                    "client_id": "fixture-attach",
                                    "lease_seconds": 55
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
                        "/api/v1/session/sess_attach_existing/orchestration/status"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "status": {
                                "main_agent_state": "runnable",
                                "waits": 0
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_attach_existing/orchestration/workers"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "workers": [
                                {
                                    "id": "main",
                                    "kind": "main_agent"
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_attach_existing/orchestration/dependencies"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "dependencies": [
                                {
                                    "from": "main",
                                    "to": "shell:bg-1",
                                    "kind": "backgroundShell",
                                    "blocking": false
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
    let attach_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "--client-id",
        "fixture-attach",
        "--lease-seconds",
        "55",
        "session-attach",
        "--session-id",
        "sess_attach_existing",
        "--thread-id",
        "thread_existing",
    ])?;
    let attach_json: Value = serde_json::from_str(&attach_output).context("parse attach output")?;
    assert_eq!(attach_json["status"], 200);
    assert_eq!(
        attach_json["body"]["session"]["session_id"],
        "sess_attach_existing"
    );

    let status_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "orchestration-status",
        "--session-id",
        "sess_attach_existing",
    ])?;
    let status_json: Value = serde_json::from_str(&status_output).context("parse status output")?;
    assert_eq!(status_json["status"], 200);
    assert_eq!(
        status_json["body"]["status"]["main_agent_state"],
        "runnable"
    );

    let workers_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "orchestration-workers",
        "--session-id",
        "sess_attach_existing",
    ])?;
    let workers_json: Value =
        serde_json::from_str(&workers_output).context("parse workers output")?;
    assert_eq!(workers_json["status"], 200);
    assert_eq!(workers_json["body"]["workers"][0]["id"], "main");

    let dependencies_output = run_broker_client(&[
        "--base-url",
        &base_url,
        "--agent-id",
        "codexw-lab",
        "orchestration-dependencies",
        "--session-id",
        "sess_attach_existing",
    ])?;
    let dependencies_json: Value =
        serde_json::from_str(&dependencies_output).context("parse dependencies output")?;
    assert_eq!(dependencies_json["status"], 200);
    assert_eq!(
        dependencies_json["body"]["dependencies"][0]["to"],
        "shell:bg-1"
    );

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
