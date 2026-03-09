use super::super::super::super::super::*;

#[test]
fn orchestration_suggest_actions_can_surface_untracked_service_contract_fixes() {
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
        .expect("start untracked service");

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
    assert!(text.contains("background_shell_list_services {\"status\":\"untracked\"}"));
    assert!(text.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}"
    ));
    assert!(text.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"label\":\"service-label\"}"
    ));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}
