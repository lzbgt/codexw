use super::super::super::super::super::*;

#[test]
fn orchestration_guidance_filter_surfaces_contract_fix_for_untracked_services() {
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

    let guidance = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "guidance"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(guidance["success"], true);
    let text = guidance["contentItems"][0]["text"]
        .as_str()
        .expect("guidance text");
    assert!(text.contains("Next action:"));
    assert!(text.contains("missing readiness or attachment metadata"));
    assert!(text.contains("background_shell_list_services {\"status\":\"untracked\"}"));
    assert!(text.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}"
    ));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}
