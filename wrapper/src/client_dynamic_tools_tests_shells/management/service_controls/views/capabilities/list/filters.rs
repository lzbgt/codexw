use super::super::super::super::super::super::*;

#[test]
fn background_shell_list_capabilities_can_filter_issue_classes() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let inspect_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_capabilities",
            "arguments": {
                "status": "missing"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(inspect_result["success"], true);
    let rendered = inspect_result["contentItems"][0]["text"]
        .as_str()
        .expect("list text");
    assert!(rendered.contains("@api.http -> <missing provider> [missing]"));
    assert!(rendered.contains("used by bg-1 [missing]"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_list_capabilities_can_filter_untracked_issue_class() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    let inspect_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_capabilities",
            "arguments": {
                "status": "untracked"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(inspect_result["success"], true);
    let rendered = inspect_result["contentItems"][0]["text"]
        .as_str()
        .expect("list text");
    assert!(rendered.contains("@api.http -> bg-1 [untracked]"));
    let _ = manager.terminate_all_running();
}
