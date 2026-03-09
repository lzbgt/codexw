use super::*;

#[test]
fn background_shell_invoke_recipe_supports_stdin_actions() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "recipes": [
                    {
                        "name": "status",
                        "description": "Ask the service for status",
                        "action": {
                            "type": "stdin",
                            "text": "status"
                        }
                    }
                ]
            }
        }),
        "/tmp",
        &manager,
    );

    let invoke_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_invoke_recipe",
            "arguments": {
                "jobId": "bg-1",
                "recipe": "status"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(invoke_result["success"], true);
    let rendered = invoke_result["contentItems"][0]["text"]
        .as_str()
        .expect("invoke text");
    assert!(rendered.contains("Action: stdin \"status\""));
    assert!(rendered.contains("Sent"));
    let _ = manager.terminate_all_running();
}
