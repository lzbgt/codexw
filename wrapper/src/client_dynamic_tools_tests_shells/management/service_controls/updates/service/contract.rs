use super::super::super::super::*;

#[test]
fn background_shell_update_service_can_update_contract_metadata() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:3000",
                "attachHint": "use /health"
            }
        }),
        "/tmp",
        &manager,
    );
    manager
        .send_input_for_operator("bg-1", "READY", true)
        .expect("send ready line");
    for _ in 0..40 {
        let rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll service");
        if rendered.contains("READY") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_service",
            "arguments": {
                "jobId": "bg-1",
                "protocol": "grpc",
                "endpoint": "grpc://127.0.0.1:50051",
                "attachHint": null,
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
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("Updated service metadata"));
    assert!(text.contains("protocol=grpc"));
    assert!(text.contains("endpoint=grpc://127.0.0.1:50051"));
    assert!(text.contains("cleared attachHint"));
    assert!(text.contains("readyPattern=READY"));
    assert!(text.contains("recipes=1"));

    let rendered = manager.attach_for_operator("bg-1").expect("attach summary");
    assert!(rendered.contains("Protocol: grpc"));
    assert!(rendered.contains("Endpoint: grpc://127.0.0.1:50051"));
    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("State: ready"));
    assert!(rendered.contains("health [http GET /health]: Check health"));
    assert!(!rendered.contains("Attach hint: use /health"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_service_reports_field_specific_validation_errors() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service"
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_service",
            "arguments": {
                "jobId": "bg-1",
                "protocol": 123
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], false);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("error text");
    assert!(text.contains("`protocol` must be a string or null"));
    let _ = manager.terminate_all_running();
}
