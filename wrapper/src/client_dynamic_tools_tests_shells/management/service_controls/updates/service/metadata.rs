use super::super::super::super::*;

#[test]
fn background_shell_update_service_can_reassign_capabilities() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
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
                "capabilities": ["frontend.dev"]
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
    assert!(text.contains("reusable capabilities=@frontend.dev"));

    let rendered = manager
        .render_service_capabilities_for_ps_filtered(None)
        .expect("capability index")
        .join("\n");
    assert!(!rendered.contains("@api.http"));
    assert!(rendered.contains("@frontend.dev"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_service_can_update_label() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
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
                "label": "frontend dev"
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
    assert!(text.contains("label=frontend dev"));

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated service");
    assert!(rendered.contains("Label: frontend dev"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_service_can_clear_capabilities_with_null() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
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
                "capabilities": null
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("cleared reusable capabilities"));

    let rendered = manager
        .render_service_capabilities_for_ps_filtered(None)
        .map(|lines| lines.join("\n"))
        .unwrap_or_default();
    assert!(!rendered.contains("@api.http"));
    let _ = manager.terminate_all_running();
}
