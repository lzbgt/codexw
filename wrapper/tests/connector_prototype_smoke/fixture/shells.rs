use super::super::*;

#[test]
fn broker_client_fixture_drives_connector_shell_job_control_workflow() -> Result<()> {
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
                    assert_eq!(body["thread_id"], "thread_jobs");
                    assert_eq!(body["client_id"], "fixture-jobs");
                    assert_eq!(body["lease_seconds"], 25);
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "session": {
                                "session_id": "sess_jobs",
                                "attachment": {
                                    "client_id": "fixture-jobs",
                                    "lease_seconds": 25
                                }
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_jobs/shells/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse shell start body")?;
                    assert_eq!(body["command"], "tail -f /tmp/app.log");
                    assert_eq!(body["label"], "tail-log");
                    assert_eq!(body["client_id"], "fixture-jobs");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "job": {
                                "job_id": "bg-7",
                                "label": "tail-log"
                            },
                            "interaction": {
                                "kind": "shell_start",
                                "job_ref": "bg-7"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_jobs/shells");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "shells": [
                                {
                                    "job_id": "bg-7",
                                    "label": "tail-log",
                                    "status": "running"
                                }
                            ]
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_jobs/shells/bg-7");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "shell": {
                                "job_id": "bg-7",
                                "label": "tail-log",
                                "status": "running"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_jobs/shells/bg-7/send");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse send body")?;
                    assert_eq!(body["text"], "status\n");
                    assert_eq!(body["client_id"], "fixture-jobs");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "interaction": {
                                "kind": "send",
                                "job_ref": "bg-7"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_jobs/shells/bg-7/poll");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse poll body")?;
                    assert_eq!(body["client_id"], "fixture-jobs");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "shell": {
                                "job_id": "bg-7",
                                "status": "running"
                            },
                            "interaction": {
                                "kind": "poll",
                                "job_ref": "bg-7"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_jobs/shells/bg-7/terminate"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse terminate body")?;
                    assert_eq!(body["client_id"], "fixture-jobs");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        json_bytes(json!({
                            "ok": true,
                            "shell": {
                                "job_id": "bg-7",
                                "status": "terminated"
                            },
                            "interaction": {
                                "kind": "terminate",
                                "job_ref": "bg-7"
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

    let base_url;
    {
        let connector_port = reserve_port()?;
        let mut connector = spawn_connector(connector_port, local_addr.port())?;
        wait_for_healthz(&mut connector, connector_port)?;
        base_url = format!("http://127.0.0.1:{connector_port}");

        let create_output = run_broker_client(&[
            "--base-url",
            &base_url,
            "--agent-id",
            "codexw-lab",
            "--client-id",
            "fixture-jobs",
            "--lease-seconds",
            "25",
            "session-create",
            "--thread-id",
            "thread_jobs",
        ])?;
        let create_json: Value =
            serde_json::from_str(&create_output).context("parse create output")?;
        assert_eq!(create_json["status"], 200);

        let shell_start_output = run_broker_client(&[
            "--base-url",
            &base_url,
            "--agent-id",
            "codexw-lab",
            "--client-id",
            "fixture-jobs",
            "shell-start",
            "--session-id",
            "sess_jobs",
            "--shell-command",
            "tail -f /tmp/app.log",
            "--label",
            "tail-log",
        ])?;
        let shell_start_json: Value =
            serde_json::from_str(&shell_start_output).context("parse shell start output")?;
        assert_eq!(shell_start_json["status"], 200);
        assert_eq!(shell_start_json["body"]["job"]["job_id"], "bg-7");

        let shells_output = run_broker_client(&[
            "--base-url",
            &base_url,
            "--agent-id",
            "codexw-lab",
            "shells",
            "--session-id",
            "sess_jobs",
        ])?;
        let shells_json: Value =
            serde_json::from_str(&shells_output).context("parse shells output")?;
        assert_eq!(shells_json["status"], 200);
        assert_eq!(shells_json["body"]["shells"][0]["job_id"], "bg-7");
        assert_eq!(shells_json["body"]["shells"][0]["label"], "tail-log");

        let shell_detail_output = run_broker_client(&[
            "--base-url",
            &base_url,
            "--agent-id",
            "codexw-lab",
            "shell-detail",
            "--session-id",
            "sess_jobs",
            "--job-ref",
            "bg-7",
        ])?;
        let shell_detail_json: Value =
            serde_json::from_str(&shell_detail_output).context("parse shell detail output")?;
        assert_eq!(shell_detail_json["status"], 200);
        assert_eq!(shell_detail_json["body"]["shell"]["label"], "tail-log");

        let send_output = run_broker_client(&[
            "--base-url",
            &base_url,
            "--agent-id",
            "codexw-lab",
            "--client-id",
            "fixture-jobs",
            "shell-send",
            "--session-id",
            "sess_jobs",
            "--job-ref",
            "bg-7",
            "--text",
            "status\n",
        ])?;
        let send_json: Value = serde_json::from_str(&send_output).context("parse send output")?;
        assert_eq!(send_json["status"], 200);
        assert_eq!(send_json["body"]["interaction"]["kind"], "send");

        let poll_output = run_broker_client(&[
            "--base-url",
            &base_url,
            "--agent-id",
            "codexw-lab",
            "--client-id",
            "fixture-jobs",
            "shell-poll",
            "--session-id",
            "sess_jobs",
            "--job-ref",
            "bg-7",
        ])?;
        let poll_json: Value = serde_json::from_str(&poll_output).context("parse poll output")?;
        assert_eq!(poll_json["status"], 200);
        assert_eq!(poll_json["body"]["shell"]["status"], "running");

        let terminate_output = run_broker_client(&[
            "--base-url",
            &base_url,
            "--agent-id",
            "codexw-lab",
            "--client-id",
            "fixture-jobs",
            "shell-terminate",
            "--session-id",
            "sess_jobs",
            "--job-ref",
            "bg-7",
        ])?;
        let terminate_json: Value =
            serde_json::from_str(&terminate_output).context("parse terminate output")?;
        assert_eq!(terminate_json["status"], 200);
        assert_eq!(terminate_json["body"]["shell"]["status"], "terminated");
    }

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
