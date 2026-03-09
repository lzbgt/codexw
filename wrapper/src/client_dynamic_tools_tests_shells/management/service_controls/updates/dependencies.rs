use super::super::super::*;

#[test]
fn background_shell_update_dependencies_can_retarget_running_job() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_dependencies",
            "arguments": {
                "jobId": "bg-1",
                "dependsOnCapabilities": ["db.redis"]
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("Updated dependency capabilities"));
    assert!(text.contains("@db.redis"));

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated blocker");
    assert!(rendered.contains("Depends on capabilities: @db.redis"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_update_dependencies_can_clear_with_null() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_update_dependencies",
            "arguments": {
                "jobId": "bg-1",
                "dependsOnCapabilities": null
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("update text");
    assert!(text.contains("Cleared dependency capabilities"));

    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll updated blocker");
    assert!(!rendered.contains("Depends on capabilities:"));
    let _ = manager.terminate_all_running();
}
