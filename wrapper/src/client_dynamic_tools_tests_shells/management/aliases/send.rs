use super::super::super::*;

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
