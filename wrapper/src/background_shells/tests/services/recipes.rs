use super::*;

#[test]
fn service_attachment_summary_exposes_endpoint_and_attach_hint() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "dev api",
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "attachHint": "Send HTTP requests to /health",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check service health",
                        "example": "curl http://127.0.0.1:4000/health",
                        "parameters": [
                            {
                                "name": "id",
                                "description": "Resource id",
                                "default": "health"
                            }
                        ],
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/{{id}}"
                        }
                    },
                    {
                        "name": "metrics",
                        "description": "Fetch metrics",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/metrics"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .attach_for_operator("bg-1")
        .expect("render attachment summary");
    assert!(rendered.contains("Service job: bg-1"));
    assert!(rendered.contains("Label: dev api"));
    assert!(rendered.contains("Protocol: http"));
    assert!(rendered.contains("Endpoint: http://127.0.0.1:4000"));
    assert!(rendered.contains("Attach hint: Send HTTP requests to /health"));
    assert!(rendered.contains("health [http GET /{{id}}]: Check service health"));
    assert!(rendered.contains("params: id=health"));
    assert!(rendered.contains("example: curl http://127.0.0.1:4000/health"));
    assert!(rendered.contains("metrics [http GET /metrics]: Fetch metrics"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_parameters_support_defaults_and_substitution() {
    let endpoint = spawn_test_http_server("GET", "/items/default-id", "ok");
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": endpoint,
                "recipes": [
                    {
                        "name": "item",
                        "description": "Fetch one item",
                        "parameters": [
                            {
                                "name": "id",
                                "default": "default-id"
                            }
                        ],
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/items/{{id}}"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator_with_args("bg-1", "item", &HashMap::new())
        .expect("invoke defaulted recipe");
    assert!(rendered.contains("Action: http GET /items/default-id"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_parameters_can_be_overridden() {
    let endpoint = spawn_test_http_server("GET", "/items/42", "ok");
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": endpoint,
                "recipes": [
                    {
                        "name": "item",
                        "description": "Fetch one item",
                        "parameters": [
                            {
                                "name": "id",
                                "required": true
                            }
                        ],
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/items/{{id}}"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator_with_args(
            "bg-1",
            "item",
            &HashMap::from([("id".to_string(), "42".to_string())]),
        )
        .expect("invoke overridden recipe");
    assert!(rendered.contains("Action: http GET /items/42"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_missing_required_parameter_fails() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": "http://127.0.0.1:4000",
                "recipes": [
                    {
                        "name": "item",
                        "description": "Fetch one item",
                        "parameters": [
                            {
                                "name": "id",
                                "required": true
                            }
                        ],
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/items/{{id}}"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "item")
        .expect_err("missing required parameter should fail");
    assert!(err.contains("parameter `id` is required"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_can_invoke_stdin_action() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": interactive_echo_command(),
                "intent": "service",
                "recipes": [
                    {
                        "name": "status",
                        "description": "Ask the service for status",
                        "action": {
                            "type": "stdin",
                            "text": "status"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "status")
        .expect("invoke stdin recipe");
    assert!(rendered.contains("Action: stdin \"status\""));
    assert!(rendered.contains("Sent"));

    let mut polled = String::new();
    for _ in 0..40 {
        polled = manager
            .poll_job("bg-1", 0, 200)
            .expect("poll shell directly");
        if polled.contains("status") {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(polled.contains("status"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_can_invoke_http_action() {
    let endpoint = spawn_test_http_server("GET", "/health", "ok");
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": endpoint,
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check service health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "health")
        .expect("invoke http recipe");
    assert!(rendered.contains("Action: http GET /health"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    assert!(rendered.contains("ok"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_http_action_waits_for_booting_service_readiness() {
    let endpoint = spawn_test_http_server("GET", "/health", "ok");
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": delayed_service_ready_command(),
                "intent": "service",
                "protocol": "http",
                "endpoint": endpoint,
                "readyPattern": "READY",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let started = std::time::Instant::now();
    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "health")
        .expect("invoke recipe after readiness wait");
    assert!(started.elapsed() >= Duration::from_millis(100));
    assert!(rendered.contains("Readiness: waited"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_can_invoke_http_action_with_headers_body_and_expected_status() {
    let endpoint = spawn_test_http_server_with_assertions(
        |request| {
            assert!(request.starts_with("POST /seed HTTP/1.1\r\n"));
            assert!(request.contains("Authorization: Bearer demo\r\n"));
            assert!(request.contains("Content-Type: application/x-www-form-urlencoded\r\n"));
            assert!(request.contains("\r\n\r\nseed=true"));
        },
        "HTTP/1.1 202 Accepted\r\nContent-Length: 6\r\nConnection: close\r\n\r\nseeded",
    );
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": endpoint,
                "recipes": [
                    {
                        "name": "seed",
                        "description": "Seed data",
                        "action": {
                            "type": "http",
                            "method": "POST",
                            "path": "/seed",
                            "body": "seed=true",
                            "headers": {
                                "Authorization": "Bearer demo",
                                "Content-Type": "application/x-www-form-urlencoded"
                            },
                            "expectedStatus": 202
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "seed")
        .expect("invoke rich http recipe");
    assert!(rendered.contains("Action: http POST /seed headers=2 body=9b expect=202"));
    assert!(rendered.contains("Status: HTTP/1.1 202 Accepted"));
    assert!(rendered.contains("Status code: 202"));
    assert!(rendered.contains("seeded"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_http_expected_status_is_enforced() {
    let endpoint = spawn_test_http_server("GET", "/health", "not-ready");
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": endpoint,
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check service health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health",
                            "expectedStatus": 204
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "health")
        .expect_err("expected status mismatch should fail");
    assert!(err.contains("expected status 204"));
    assert!(err.contains("Status code: 200"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_can_invoke_tcp_action() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]).into_owned();
        assert_eq!(request, "PING\n");
        stream.write_all(b"PONG\n").expect("write response");
        stream.flush().expect("flush response");
    });
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "tcp",
                "endpoint": format!("tcp://{addr}"),
                "recipes": [
                    {
                        "name": "ping",
                        "description": "Ping the raw socket service",
                        "action": {
                            "type": "tcp",
                            "payload": "PING",
                            "appendNewline": true,
                            "expectSubstring": "PONG",
                            "readTimeoutMs": 500
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start tcp service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "ping")
        .expect("invoke tcp recipe");
    assert!(
        rendered.contains("Action: tcp payload=\"PING\" newline expect=\"PONG\" timeout=500ms")
    );
    assert!(rendered.contains("Address:"));
    assert!(rendered.contains("Payload:"));
    assert!(rendered.contains("PING"));
    assert!(rendered.contains("PONG"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_tcp_expectation_is_enforced() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).expect("read request");
        stream.write_all(b"ERR\n").expect("write response");
        stream.flush().expect("flush response");
    });
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "tcp",
                "endpoint": format!("{addr}"),
                "recipes": [
                    {
                        "name": "ping",
                        "description": "Ping the raw socket service",
                        "action": {
                            "type": "tcp",
                            "payload": "PING",
                            "appendNewline": true,
                            "expectSubstring": "PONG",
                            "readTimeoutMs": 500
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start tcp service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "ping")
        .expect_err("expectation mismatch should fail");
    assert!(err.contains("expected substring `PONG`"));
    assert!(err.contains("ERR"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_can_invoke_redis_action() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]).into_owned();
        assert_eq!(request, "*1\r\n$4\r\nPING\r\n");
        stream.write_all(b"+PONG\r\n").expect("write response");
        stream.flush().expect("flush response");
    });
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "redis",
                "endpoint": format!("tcp://{addr}"),
                "recipes": [
                    {
                        "name": "ping",
                        "description": "Ping the redis service",
                        "action": {
                            "type": "redis",
                            "command": ["PING"],
                            "expectSubstring": "PONG",
                            "readTimeoutMs": 500
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start redis service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "ping")
        .expect("invoke redis recipe");
    assert!(rendered.contains("Action: redis PING expect=\"PONG\" timeout=500ms"));
    assert!(rendered.contains("Command: PING"));
    assert!(rendered.contains("Type: simple-string"));
    assert!(rendered.contains("Value: PONG"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_recipe_redis_expectation_is_enforced() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).expect("read request");
        stream.write_all(b"+NOPE\r\n").expect("write response");
        stream.flush().expect("flush response");
    });
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "redis",
                "endpoint": format!("{addr}"),
                "recipes": [
                    {
                        "name": "ping",
                        "description": "Ping the redis service",
                        "action": {
                            "type": "redis",
                            "command": ["PING"],
                            "expectSubstring": "PONG",
                            "readTimeoutMs": 500
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start redis service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "ping")
        .expect_err("expectation mismatch should fail");
    assert!(err.contains("expected substring `PONG`"));
    assert!(err.contains("Value: NOPE"));
    let _ = manager.terminate_all_running();
}

#[test]
fn informational_recipe_cannot_be_invoked() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "recipes": [
                    {
                        "name": "docs",
                        "description": "Read the service docs first"
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start service shell");

    let err = manager
        .invoke_recipe_for_operator("bg-1", "docs")
        .expect_err("informational recipe should not be invokable");
    assert!(err.contains("descriptive only"));
    let _ = manager.terminate_all_running();
}

#[test]
fn service_attachment_fields_require_service_intent() {
    let manager = BackgroundShellManager::default();
    let err = manager
        .start_from_tool(
            &json!({
                "command": "sleep 0.1",
                "intent": "observation",
                "protocol": "http"
            }),
            "/tmp",
        )
        .expect_err("service attachment fields should require service intent");
    assert!(err.contains("service contract fields"));
}
