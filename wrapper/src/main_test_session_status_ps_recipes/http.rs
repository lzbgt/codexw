use super::super::*;

#[test]
fn ps_command_can_invoke_service_recipe_for_aliased_job() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        for _ in 0..2 {
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
        }
    });

    let cli = test_cli();
    let mut state = crate::state::AppState::new(true, false);
    let mut output = Output::default();
    let mut writer = spawn_sink_stdin();
    state
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
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
            }),
            "/tmp",
        )
        .expect("start service shell");

    handle_ps_command(
        "alias 1 dev.api",
        &["alias", "1", "dev.api"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("alias background shell");

    handle_ps_command(
        "run dev.api health",
        &["run", "dev.api", "health"],
        &cli,
        &mut state,
        &mut output,
        &mut writer,
    )
    .expect("invoke service recipe");

    let rendered = state
        .background_shells
        .invoke_recipe_for_operator("bg-1", "health")
        .expect("invoke recipe directly after command path");
    assert!(rendered.contains("Action: http GET /health"));
    assert!(rendered.contains("HTTP/1.1 200 OK"));
    let _ = state.background_shells.terminate_all_running();
}
