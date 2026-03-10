use super::*;

#[test]
fn connector_broker_style_workflow_covers_shell_and_service_control() -> Result<()> {
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
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/shells/start");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse shell start body")?;
                    assert_eq!(body["command"], "npm run dev");
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "job": { "job_id": "bg-1", "alias": "dev.api" }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "services": [
                                {
                                    "job_id": "bg-1",
                                    "alias": "dev.api",
                                    "ready_state": "ready",
                                    "endpoint": "http://127.0.0.1:8080"
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
                        "/api/v1/session/sess_1/services/dev.api/attach"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse attach body")?;
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": { "job_id": "bg-1", "alias": "dev.api" },
                            "attachment_text": "curl http://127.0.0.1:8080/health"
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api/wait");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse wait body")?;
                    assert_eq!(body["timeout_ms"], 5000);
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "alias": "dev.api",
                                "ready_state": "ready"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api/run");
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse run body")?;
                    assert_eq!(body["recipe"], "health");
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "recipe": { "name": "health" },
                            "result": "healthy"
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/capabilities");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "capabilities": [
                                {
                                    "name": "@api.http",
                                    "status": "healthy",
                                    "providers": ["bg-1"]
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

    let client = BrokerClient::new(connector_port, "codexw-lab");
    let create_response = client.create_session(
        "{\"thread_id\":\"thread_1\"}",
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
            ("X-Codexw-Lease-Seconds", "45"),
        ],
    )?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));

    let shell_response = client.session_request(
        "POST",
        "sess_1",
        "/shells",
        Some("{\"command\":\"npm run dev\"}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(shell_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(shell_response.contains("\"job_id\":\"bg-1\""));

    let services_response = client.session_request("GET", "sess_1", "/services", None, &[])?;
    assert!(services_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(services_response.contains("\"alias\":\"dev.api\""));

    let attach_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/attach",
        Some("{}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(attach_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(attach_response.contains("curl http://127.0.0.1:8080/health"));

    let wait_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/wait",
        Some("{\"timeout_ms\":5000}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(wait_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(wait_response.contains("\"ready_state\":\"ready\""));

    let run_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/run",
        Some("{\"recipe\":\"health\"}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(run_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(run_response.contains("\"name\":\"health\""));
    assert!(run_response.contains("\"result\":\"healthy\""));

    let capabilities_response =
        client.session_request("GET", "sess_1", "/capabilities", None, &[])?;
    assert!(capabilities_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(capabilities_response.contains("\"name\":\"@api.http\""));
    assert!(capabilities_response.contains("\"providers\":[\"bg-1\"]"));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}

#[test]
fn connector_broker_style_workflow_covers_service_mutations() -> Result<()> {
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
                }
                1 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/provide"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse provide body")?;
                    assert_eq!(body["capabilities"], json!(["@api.http", "@frontend.dev"]));
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "alias": "dev.api",
                                "capabilities": ["@api.http", "@frontend.dev"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                2 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/depend"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse depend body")?;
                    assert_eq!(body["depends_on_capabilities"], json!(["@db.primary"]));
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "depends_on_capabilities": ["@db.primary"]
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                3 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/contract"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse contract body")?;
                    assert_eq!(body["protocol"], "http");
                    assert_eq!(body["endpoint"], "http://127.0.0.1:8080");
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "protocol": "http",
                                "endpoint": "http://127.0.0.1:8080"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                4 => {
                    assert_eq!(request.method, "POST");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/services/dev.api/relabel"
                    );
                    let body: Value =
                        serde_json::from_slice(&request.body).context("parse relabel body")?;
                    assert_eq!(body["label"], "Frontend dev service");
                    assert_eq!(body["client_id"], "remote-web");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "label": "Frontend dev service"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                5 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(request.path, "/api/v1/session/sess_1/services/dev.api");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "service": {
                                "job_id": "bg-1",
                                "alias": "dev.api",
                                "label": "Frontend dev service",
                                "capabilities": ["@api.http", "@frontend.dev"],
                                "depends_on_capabilities": ["@db.primary"],
                                "protocol": "http",
                                "endpoint": "http://127.0.0.1:8080"
                            }
                        }))?
                        .as_slice(),
                    )?;
                }
                6 => {
                    assert_eq!(request.method, "GET");
                    assert_eq!(
                        request.path,
                        "/api/v1/session/sess_1/capabilities/@frontend.dev"
                    );
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &[("Content-Type", "application/json")],
                        serde_json::to_vec(&json!({
                            "ok": true,
                            "capability": {
                                "name": "@frontend.dev",
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

    let client = BrokerClient::new(connector_port, "codexw-lab");
    let create_response = client.create_session(
        "{\"thread_id\":\"thread_1\"}",
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
            ("X-Codexw-Lease-Seconds", "45"),
        ],
    )?;
    assert!(create_response.starts_with("HTTP/1.1 200 OK\r\n"));

    let provide_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/provide",
        Some("{\"capabilities\":[\"@api.http\",\"@frontend.dev\"]}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(provide_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(provide_response.contains("\"@frontend.dev\""));

    let depend_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/depend",
        Some("{\"depends_on_capabilities\":[\"@db.primary\"]}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(depend_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(depend_response.contains("\"@db.primary\""));

    let contract_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/contract",
        Some("{\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:8080\"}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(contract_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(contract_response.contains("\"protocol\":\"http\""));

    let relabel_response = client.session_request(
        "POST",
        "sess_1",
        "/services/dev.api/relabel",
        Some("{\"label\":\"Frontend dev service\"}"),
        &[
            ("Content-Type", "application/json"),
            ("X-Codexw-Client-Id", "remote-web"),
        ],
    )?;
    assert!(relabel_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(relabel_response.contains("\"Frontend dev service\""));

    let service_response =
        client.session_request("GET", "sess_1", "/services/dev.api", None, &[])?;
    assert!(service_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(service_response.contains("\"label\":\"Frontend dev service\""));
    assert!(service_response.contains("\"@frontend.dev\""));
    assert!(service_response.contains("\"@db.primary\""));

    let capability_response =
        client.session_request("GET", "sess_1", "/capabilities/%40frontend.dev", None, &[])?;
    assert!(capability_response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(capability_response.contains("\"name\":\"@frontend.dev\""));
    assert!(capability_response.contains("\"providers\":[\"bg-1\"]"));

    fake_server.join().expect("fake server thread")?;
    Ok(())
}
