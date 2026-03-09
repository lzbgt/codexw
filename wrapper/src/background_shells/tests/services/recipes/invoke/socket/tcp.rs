use super::super::super::super::*;

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
