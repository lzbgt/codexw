use super::super::super::*;

#[test]
fn orchestration_suggest_actions_can_focus_one_capability() {
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
            "tool": "orchestration_suggest_actions",
            "arguments": {
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("focused actions text");
    assert!(text.contains("Suggested actions (@api.http):"));
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

#[test]
fn orchestration_suggest_actions_can_focus_untracked_capability_contract_fixes() {
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
        .expect("start untracked provider");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_suggest_actions",
            "arguments": {
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("focused actions text");
    assert!(text.contains("Suggested actions (@api.http):"));
    assert!(text.contains(
        "background_shell_list_services {\"status\":\"untracked\",\"capability\":\"@api.http\"}"
    ));
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
fn orchestration_suggest_actions_can_focus_ambiguous_capability_with_non_conflicting_fix() {
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
            "tool": "orchestration_suggest_actions",
            "arguments": {
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("focused actions text");
    assert!(text.contains("Suggested actions (@api.http):"));
    assert!(text.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":[\"@other.role\"]}"
    ));
    assert!(
        text.contains("background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":null}")
    );
    assert!(!text.contains(
        "background_shell_update_service {\"jobId\":\"<jobId|alias|n>\",\"capabilities\":[\"@api.http\"]}"
    ));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}
