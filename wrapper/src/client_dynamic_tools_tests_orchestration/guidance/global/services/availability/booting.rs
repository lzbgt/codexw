use super::super::super::super::super::*;

#[test]
fn orchestration_guidance_filter_uses_concrete_wait_for_single_booting_service() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "capabilities": ["api.http"],
                "readyPattern": "READY"
            }),
            "/tmp",
        )
        .expect("start booting service");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "guidance"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("guidance text");
    assert!(text.contains("background_shell_list_services {\"status\":\"booting\"}"));
    assert!(text.contains("background_shell_wait_ready {\"jobId\":\"bg-1\",\"timeoutMs\":5000}"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}
