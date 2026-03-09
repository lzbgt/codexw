use super::super::super::super::super::*;

#[test]
fn running_service_contract_can_be_updated_without_restart() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:3000",
                "attachHint": "use /health"
            }),
            "/tmp",
        )
        .expect("start service");
    manager
        .send_input_for_operator("bg-1", "READY", true)
        .expect("send ready line");
    let mut rendered = String::new();
    for _ in 0..40 {
        rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll service output");
        if rendered.contains("READY") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(rendered.contains("READY"));

    manager
        .set_running_service_contract(
            "bg-1",
            Some(Some("grpc".to_string())),
            Some(Some("grpc://127.0.0.1:50051".to_string())),
            Some(None),
            Some(Some("READY".to_string())),
            Some(vec![
                crate::background_shells::BackgroundShellInteractionRecipe {
                    name: "health".to_string(),
                    description: Some("Check health".to_string()),
                    example: None,
                    parameters: Vec::new(),
                    action: crate::background_shells::BackgroundShellInteractionAction::Http {
                        method: "GET".to_string(),
                        path: "/health".to_string(),
                        body: None,
                        headers: Vec::new(),
                        expected_status: None,
                    },
                },
            ]),
        )
        .expect("update contract");

    let rendered = manager.attach_for_operator("bg-1").expect("attach summary");
    assert!(rendered.contains("Protocol: grpc"));
    assert!(rendered.contains("Endpoint: grpc://127.0.0.1:50051"));
    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("State: ready"));
    assert!(rendered.contains("health [http GET /health]: Check health"));
    assert!(!rendered.contains("Attach hint: use /health"));
    let _ = manager.terminate_all_running();
}
