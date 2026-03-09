use super::super::super::BackgroundShellManager;
use super::super::super::execute_dynamic_tool_call;
use super::super::super::json;

#[test]
fn background_shell_invoke_recipe_waits_for_ready_pattern_before_http_call() {
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
                "command": if cfg!(windows) {
                    "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "sleep 0.15; printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "protocol": "http",
                "endpoint": format!("http://{addr}"),
                "readyPattern": "READY",
                "recipes": [
                    {
                        "name": "health",
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

    let started = std::time::Instant::now();
    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "health",
                "waitForReadyMs": 2_000
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    assert!(started.elapsed() >= std::time::Duration::from_millis(100));
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Readiness: waited"));
    assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
    let _ = manager.terminate_all_running();
}
