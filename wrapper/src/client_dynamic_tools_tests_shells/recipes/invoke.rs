use super::super::BackgroundShellManager;
use super::super::execute_dynamic_tool_call;
use super::super::json;

#[test]
fn background_shell_invoke_recipe_runs_structured_service_action() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert!(request.starts_with("GET /health HTTP/1.1\r\n"));
        std::io::Write::write_all(
            &mut stream,
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        )
        .expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.health"],
                "protocol": "http",
                "endpoint": format!("http://{addr}"),
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
            }
        }),
        "/tmp",
        &manager,
    );
    manager.set_job_alias("bg-1", "dev.api").expect("set alias");

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "@api.health",
                "recipe": "health"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Action: http GET /health"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_invoke_recipe_supports_http_headers_body_and_expected_status() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert!(request.starts_with("POST /seed HTTP/1.1\r\n"));
        assert!(request.contains("Authorization: Bearer demo\r\n"));
        assert!(request.contains("\r\n\r\nseed=true"));
        std::io::Write::write_all(
            &mut stream,
            b"HTTP/1.1 202 Accepted\r\nContent-Length: 7\r\nConnection: close\r\n\r\nseeded!",
        )
        .expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "http",
                "endpoint": format!("http://{addr}"),
                "recipes": [
                    {
                        "name": "seed",
                        "description": "Seed the service",
                        "action": {
                            "type": "http",
                            "method": "POST",
                            "path": "/seed",
                            "body": "seed=true",
                            "headers": {
                                "Authorization": "Bearer demo"
                            },
                            "expectedStatus": 202
                        }
                    }
                ]
            }
        }),
        "/tmp",
        &manager,
    );

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "seed"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Action: http POST /seed headers=1 body=9b expect=202"));
    assert!(rendered.contains("Status code: 202"));
    assert!(rendered.contains("seeded!"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_invoke_recipe_supports_tcp_actions() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert_eq!(request, "PING\n");
        std::io::Write::write_all(&mut stream, b"PONG\n").expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
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
            }
        }),
        "/tmp",
        &manager,
    );

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "ping"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(
        rendered.contains("Action: tcp payload=\"PING\" newline expect=\"PONG\" timeout=500ms")
    );
    assert!(rendered.contains("Address:"));
    assert!(rendered.contains("PONG"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_invoke_recipe_supports_redis_actions() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert_eq!(request, "*1\r\n$4\r\nPING\r\n");
        std::io::Write::write_all(&mut stream, b"+PONG\r\n").expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
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
            }
        }),
        "/tmp",
        &manager,
    );

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "ping"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Action: redis PING expect=\"PONG\" timeout=500ms"));
    assert!(rendered.contains("Type: simple-string"));
    assert!(rendered.contains("Value: PONG"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_invoke_recipe_supports_parameter_args() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0_u8; 4096];
        let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..bytes]);
        assert_eq!(request, "*2\r\n$3\r\nGET\r\n$5\r\nalpha\r\n");
        std::io::Write::write_all(&mut stream, b"$5\r\nvalue\r\n").expect("write response");
    });

    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "protocol": "redis",
                "endpoint": format!("tcp://{addr}"),
                "recipes": [
                    {
                        "name": "get",
                        "description": "Get one cache entry",
                        "parameters": [
                            {
                                "name": "key",
                                "required": true
                            }
                        ],
                        "action": {
                            "type": "redis",
                            "command": ["GET", "{{key}}"],
                            "expectSubstring": "value",
                            "readTimeoutMs": 500
                        }
                    }
                ]
            }
        }),
        "/tmp",
        &manager,
    );

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "get",
                "args": {
                    "key": "alpha"
                }
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Action: redis GET alpha"));
    assert!(rendered.contains("Value: value"));
    let _ = manager.terminate_all_running();
}
