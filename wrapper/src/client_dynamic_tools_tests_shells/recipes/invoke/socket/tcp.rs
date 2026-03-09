use super::super::*;

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
