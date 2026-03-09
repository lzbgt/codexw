use super::super::super::*;

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

#[test]
fn orchestration_suggest_actions_can_use_concrete_wait_for_single_booting_service() {
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
    assert!(text.contains("background_shell_list_services {\"status\":\"booting\"}"));
    assert!(text.contains("background_shell_wait_ready {\"jobId\":\"bg-1\",\"timeoutMs\":5000}"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

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
