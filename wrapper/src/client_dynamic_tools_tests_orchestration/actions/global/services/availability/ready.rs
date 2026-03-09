use super::super::super::super::super::*;

#[test]
fn orchestration_suggest_actions_can_use_concrete_provider_for_single_ready_service() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "printf 'READY\\n'; sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
                "readyPattern": "READY",
                "recipes": [{
                    "name": "health",
                    "action": {
                        "type": "stdin",
                        "text": "health"
                    }
                }]
            }),
            "/tmp",
        )
        .expect("start ready service");
    state
        .orchestration
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait ready service");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_suggest_actions"
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("actions text");
    assert!(text.contains("Suggested actions:"));
    assert!(text.contains("background_shell_attach {\"jobId\":\"bg-1\"}"));
    assert!(
        text.contains("background_shell_invoke_recipe {\"jobId\":\"bg-1\",\"recipe\":\"health\"}")
    );
    assert!(!text.contains("\"args\":"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}
