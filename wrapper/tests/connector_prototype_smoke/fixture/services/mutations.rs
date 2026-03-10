use super::*;

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
