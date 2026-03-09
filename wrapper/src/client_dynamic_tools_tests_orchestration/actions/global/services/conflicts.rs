use super::super::super::super::*;

#[test]
fn orchestration_suggest_actions_returns_concrete_tool_steps() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first provider");
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second provider");

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
    assert!(text.contains("background_shell_inspect_capability {\"capability\":\"@api.http\"}"));
    assert!(text.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":[\"@other.role\"]}"
    ));
    assert!(
        text.contains("background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":null}")
    );
    assert!(
        text.contains(
            "background_shell_clean {\"scope\":\"services\",\"capability\":\"@api.http\"}"
        )
    );
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}
