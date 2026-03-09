use super::super::super::super::*;

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
