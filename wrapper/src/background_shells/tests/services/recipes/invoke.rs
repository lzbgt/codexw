use super::super::*;

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
        stream
            .write_all(b"$5\r\nwrong\r\n")
            .expect("write response");
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

    let err = manager
        .invoke_recipe_for_operator("bg-1", "ping")
        .expect_err("expectation mismatch should fail");
    assert!(err.contains("expected substring `PONG`"));
    assert!(err.contains("Value: wrong"));
    let _ = manager.terminate_all_running();
}
