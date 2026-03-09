use super::super::*;

#[test]
fn orchestration_list_workers_guidance_can_focus_one_capability() {
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
        .expect("start api provider");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "guidance",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("focused guidance text");
    assert!(text.contains("Next action (@api.http):"));
    assert!(text.contains("untracked service"));
    assert!(text.contains(
        "background_shell_list_services {\"status\":\"untracked\",\"capability\":\"@api.http\"}"
    ));
    assert!(text.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}"
    ));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}
