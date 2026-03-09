use super::super::*;

#[test]
fn background_shell_send_writes_to_alias_target() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "label": "echo shell"
            }
        }),
        "/tmp",
        &manager,
    );
    manager.set_job_alias("bg-1", "dev.api").expect("set alias");

    let send_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_send",
            "arguments": {
                "jobId": "dev.api",
                "text": "ping from tool"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(send_result["success"], true);
    let mut rendered = String::new();
    for _ in 0..40 {
        let poll_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_poll",
                "arguments": {
                    "jobId": "dev.api"
                }
            }),
            "/tmp",
            &manager,
        );
        rendered = poll_result["contentItems"][0]["text"]
            .as_str()
            .expect("poll text")
            .to_string();
        if rendered.contains("ping from tool") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    assert!(rendered.contains("ping from tool"));
    let _ = manager.terminate_all_running();
}

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
