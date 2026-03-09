use super::*;

#[test]
fn dynamic_tool_specs_include_workspace_tools() {
    let specs = dynamic_tool_specs();
    let names = specs
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "orchestration_status",
            "orchestration_list_workers",
            "orchestration_suggest_actions",
            "orchestration_list_dependencies",
            "workspace_list_dir",
            "workspace_stat_path",
            "workspace_read_file",
            "workspace_find_files",
            "workspace_search_text",
            "background_shell_start",
            "background_shell_poll",
            "background_shell_send",
            "background_shell_set_alias",
            "background_shell_list_capabilities",
            "background_shell_list_services",
            "background_shell_update_service",
            "background_shell_update_dependencies",
            "background_shell_inspect_capability",
            "background_shell_attach",
            "background_shell_wait_ready",
            "background_shell_invoke_recipe",
            "background_shell_list",
            "background_shell_terminate",
            "background_shell_clean"
        ]
    );
}

#[test]
fn orchestration_status_reports_worker_and_guidance_summary() {
    let state = AppState::new(true, false);
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
        .expect("start dependent shell");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_status"
        }),
        "/tmp",
        &state,
    );

    assert_eq!(result["success"], true);
    let rendered = result["contentItems"][0]["text"]
        .as_str()
        .expect("status text");
    assert!(rendered.contains("orchestration   main=1"));
    assert!(rendered.contains("cap_deps_missing=1"));
    assert!(rendered.contains("next action"));
    assert!(rendered.contains("missing service capability @api.http"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_list_workers_supports_filtered_capability_and_guidance_views() {
    let state = AppState::new(true, false);
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
        .expect("start dependent shell");

    let caps = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "capabilities"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(caps["success"], true);
    let caps_text = caps["contentItems"][0]["text"]
        .as_str()
        .expect("capabilities text");
    assert!(caps_text.contains("Service capability index:"));
    assert!(caps_text.contains("@api.http -> <missing provider> [missing]"));

    let deps = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "dependencies"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(deps["success"], true);
    let deps_text = deps["contentItems"][0]["text"]
        .as_str()
        .expect("dependency text");
    assert!(deps_text.contains("Dependencies:"));
    assert!(!deps_text.contains("Main agent state:"));
    assert!(deps_text.contains("shell:bg-1 -> capability:@api.http"));

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
    assert!(guidance_text.contains("missing service capability @api.http"));

    let actions = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "actions"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(actions["success"], true);
    let actions_text = actions["contentItems"][0]["text"]
        .as_str()
        .expect("actions text");
    assert!(actions_text.contains("Suggested actions:"));
    assert!(
        actions_text.contains("background_shell_inspect_capability {\"capability\":\"@api.http\"}")
    );
    assert!(actions_text.contains(
        "background_shell_update_service {\"jobId\":\"<jobId|alias|n>\",\"capabilities\":[\"@api.http\"]}"
    ));
    assert!(actions_text.contains(
        "background_shell_update_dependencies {\"jobId\":\"bg-1\",\"dependsOnCapabilities\":[\"@other.role\"]}"
    ));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

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
                "readyPattern": "READY"
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
                "readyPattern": "READY"
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
        text.contains("background_shell_invoke_recipe {\"jobId\":\"bg-1\",\"recipe\":\"...\"}")
    );
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

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
    assert!(text.contains("/ps services untracked @api.http"));
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

#[test]
fn orchestration_list_workers_blockers_can_focus_one_capability() {
    let state = AppState::new(true, false);
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
        .expect("start api blocker");
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db blocker");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "blockers",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("focused blockers text");
    assert!(text.contains("Dependencies (@api.http):"));
    assert!(text.contains("shell:bg-1 -> capability:@api.http"));
    assert!(!text.contains("db.redis"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_list_workers_rejects_capability_for_non_focus_filters() {
    let state = AppState::new(true, false);
    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "services",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error")
            .contains(
                "only supported with `filter=blockers`, `filter=guidance`, or `filter=actions`"
            )
    );
}

#[test]
fn orchestration_list_dependencies_supports_issue_filters() {
    let state = AppState::new(true, false);
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
        .expect("start dependent shell");

    let missing = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "filter": "missing"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(missing["success"], true);
    let missing_text = missing["contentItems"][0]["text"]
        .as_str()
        .expect("missing dependency text");
    assert!(missing_text.contains("Dependencies:"));
    assert!(
        missing_text.contains(
            "shell:bg-1 -> capability:@api.http  [dependsOnCapability:missing, blocking]"
        )
    );

    let sidecars = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "filter": "sidecars"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(sidecars["success"], true);
    let sidecar_text = sidecars["contentItems"][0]["text"]
        .as_str()
        .expect("sidecar dependency text");
    assert!(sidecar_text.contains("No sidecar dependency edges tracked right now."));

    let focused = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "filter": "missing",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(focused["success"], true);
    let focused_text = focused["contentItems"][0]["text"]
        .as_str()
        .expect("focused dependency text");
    assert!(focused_text.contains("Dependencies (@api.http):"));
    assert!(
        focused_text.contains(
            "shell:bg-1 -> capability:@api.http  [dependsOnCapability:missing, blocking]"
        )
    );
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn orchestration_list_workers_rejects_unknown_filters() {
    let state = AppState::new(true, false);
    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_workers",
            "arguments": {
                "filter": "weird"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error")
            .contains("orchestration_list_workers `filter`")
    );
}

#[test]
fn orchestration_list_dependencies_rejects_unknown_filters() {
    let state = AppState::new(true, false);
    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "filter": "weird"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error")
            .contains("orchestration_list_dependencies `filter`")
    );
}

#[test]
fn orchestration_list_dependencies_rejects_empty_capability_argument() {
    let state = AppState::new(true, false);
    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "orchestration_list_dependencies",
            "arguments": {
                "capability": "@"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error")
            .contains("orchestration_list_dependencies `capability`")
    );
}
