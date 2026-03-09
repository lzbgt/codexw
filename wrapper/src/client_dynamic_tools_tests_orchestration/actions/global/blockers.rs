use super::super::super::*;

#[test]
fn orchestration_suggest_actions_can_use_concrete_wait_for_booting_blocker_provider() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
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
        .expect("start booting service");
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start blocked prerequisite");

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
    assert!(text.contains(
        "background_shell_list_services {\"status\":\"booting\",\"capability\":\"@api.http\"}"
    ));
    assert!(text.contains("background_shell_wait_ready {\"jobId\":\"bg-1\",\"timeoutMs\":5000}"));
    assert!(text.contains(
        "orchestration_list_dependencies {\"filter\":\"booting\",\"capability\":\"@api.http\"}"
    ));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_suggest_actions_can_use_concrete_poll_for_single_generic_blocker() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite"
            }),
            "/tmp",
        )
        .expect("start generic blocker");

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
    assert!(text.contains("orchestration_list_workers {\"filter\":\"blockers\"}"));
    assert!(text.contains("background_shell_poll {\"jobId\":\"bg-1\"}"));
    assert!(text.contains("background_shell_clean {\"scope\":\"blockers\"}"));
    assert!(!text.contains("background_shell_wait_ready"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}
