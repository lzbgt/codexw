use super::super::super::*;

#[test]
fn background_shell_set_alias_can_assign_and_clear_alias() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "observation"
            }
        }),
        "/tmp",
        &manager,
    );

    let assign_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_set_alias",
            "arguments": {
                "jobId": "bg-1",
                "alias": "dev.api"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(assign_result["success"], true);
    assert!(
        assign_result["contentItems"][0]["text"]
            .as_str()
            .expect("assign text")
            .contains("Aliased background shell job bg-1 as dev.api")
    );
    assert_eq!(
        manager
            .resolve_job_reference("dev.api")
            .expect("resolve alias"),
        "bg-1"
    );

    let clear_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_set_alias",
            "arguments": {
                "jobId": "dev.api",
                "alias": null
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(clear_result["success"], true);
    assert!(
        clear_result["contentItems"][0]["text"]
            .as_str()
            .expect("clear text")
            .contains("Cleared alias for background shell job bg-1")
    );
    assert!(manager.resolve_job_reference("dev.api").is_err());
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_set_alias_reports_validation_errors() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "observation"
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_set_alias",
            "arguments": {
                "jobId": "bg-1",
                "alias": 123
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error text")
            .contains("`alias` must be a string or null")
    );
    let _ = manager.terminate_all_running();
}
