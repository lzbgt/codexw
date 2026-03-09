use super::super::super::*;

#[test]
fn service_recipe_can_invoke_stdin_action() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({
                "command": interactive_echo_command(),
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
            }),
            "/tmp",
        )
        .expect("start service shell");

    let rendered = manager
        .invoke_recipe_for_operator("bg-1", "status")
        .expect("invoke stdin recipe");
    assert!(rendered.contains("Action: stdin \"status\""));
    assert!(rendered.contains("Sent"));

    let mut polled = String::new();
    for _ in 0..40 {
        polled = manager
            .poll_job("bg-1", 0, 200)
            .expect("poll shell directly");
        if polled.contains("status") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(polled.contains("status"));
    let _ = manager.terminate_all_running();
}
