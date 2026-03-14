use super::super::super::*;

#[test]
fn background_shell_poll_returns_terminal_cursor_exhaustion_as_tool_failure() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(&json!({"command": "printf 'alpha\\n'; exit 2"}), "/tmp")
        .expect("start failing background shell");

    for _ in 0..20 {
        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_poll",
                "arguments": {"jobId": "bg-1"}
            }),
            "/tmp",
            &manager,
        );
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("poll text");
        if text.contains("Next afterLine: 1") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_poll",
            "arguments": {"jobId": "bg-1", "afterLine": 1}
        }),
        "/tmp",
        &manager,
    );
    assert_eq!(result["success"], false);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("error text");
    assert!(text.contains("terminal state (failed with exit code 2)"));
    assert!(text.contains("Stop polling this job"));
}
