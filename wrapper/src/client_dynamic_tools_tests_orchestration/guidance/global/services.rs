use super::super::super::*;

#[test]
fn orchestration_guidance_filter_uses_concrete_provider_ref_for_single_ready_service() {
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
        .expect("wait for ready service");

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
    let guidance_text = guidance["contentItems"][0]["text"]
        .as_str()
        .expect("guidance text");
    assert!(guidance_text.contains("background_shell_attach {\"jobId\":\"bg-1\"}"));
    assert!(
        guidance_text
            .contains("background_shell_invoke_recipe {\"jobId\":\"bg-1\",\"recipe\":\"health\"}")
    );
    assert!(!guidance_text.contains("\"args\":"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_guidance_filter_omits_invoke_for_descriptive_only_ready_service() {
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
                    "name": "docs",
                    "description": "Read usage notes"
                }]
            }),
            "/tmp",
        )
        .expect("start ready service with descriptive recipe");
    state
        .orchestration
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait for ready service");

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
    let guidance_text = guidance["contentItems"][0]["text"]
        .as_str()
        .expect("guidance text");
    assert!(guidance_text.contains("background_shell_attach {\"jobId\":\"bg-1\"}"));
    assert!(!guidance_text.contains("background_shell_invoke_recipe"));

    let actions = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_suggest_actions"
        }),
        "/tmp",
        &state,
    );
    assert_eq!(actions["success"], true);
    let actions_text = actions["contentItems"][0]["text"]
        .as_str()
        .expect("actions text");
    assert!(actions_text.contains("background_shell_attach {\"jobId\":\"bg-1\"}"));
    assert!(!actions_text.contains("background_shell_invoke_recipe"));

    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_guidance_prefers_health_recipe_over_earlier_generic_recipe() {
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
                "recipes": [
                    {
                        "name": "query",
                        "action": {
                            "type": "stdin",
                            "text": "query {{key}}"
                        },
                        "parameters": [{"name": "key", "required": true}]
                    },
                    {
                        "name": "health",
                        "action": {
                            "type": "stdin",
                            "text": "health"
                        }
                    }
                ]
            }),
            "/tmp",
        )
        .expect("start ready service");
    state
        .orchestration
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait for ready service");

    let actions = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_suggest_actions"
        }),
        "/tmp",
        &state,
    );
    assert_eq!(actions["success"], true);
    let text = actions["contentItems"][0]["text"]
        .as_str()
        .expect("actions text");
    assert!(
        text.contains("background_shell_invoke_recipe {\"jobId\":\"bg-1\",\"recipe\":\"health\"}")
    );
    assert!(!text.contains("\"recipe\":\"query\""));

    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_guidance_includes_example_args_for_parameterized_recipe() {
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
                    "name": "query",
                    "action": {
                        "type": "stdin",
                        "text": "query {{key}} {{mode}}"
                    },
                    "parameters": [
                        {"name": "key", "required": true},
                        {"name": "mode", "default": "fast"}
                    ]
                }]
            }),
            "/tmp",
        )
        .expect("start ready service");
    state
        .orchestration
        .background_shells
        .wait_ready_for_operator("bg-1", 1000)
        .expect("wait for ready service");

    let actions = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_suggest_actions"
        }),
        "/tmp",
        &state,
    );
    assert_eq!(actions["success"], true);
    let text = actions["contentItems"][0]["text"]
        .as_str()
        .expect("actions text");
    assert!(text.contains(
        "background_shell_invoke_recipe {\"jobId\":\"bg-1\",\"recipe\":\"query\",\"args\":{\"key\":\"value\",\"mode\":\"fast\"}}"
    ));

    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

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
    let guidance_text = guidance["contentItems"][0]["text"]
        .as_str()
        .expect("guidance text");
    assert!(guidance_text.contains("background_shell_list_services {\"status\":\"untracked\"}"));
    assert!(guidance_text.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}"
    ));
    assert!(guidance_text.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"label\":\"service-label\"}"
    ));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}
