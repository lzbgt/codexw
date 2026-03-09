use super::super::super::*;

#[test]
fn orchestration_status_reports_worker_and_next_action_summary() {
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
    assert!(
        rendered.contains("background_shell_inspect_capability {\"capability\":\"@api.http\"}")
    );
    assert!(!rendered.contains(":ps provide"));
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
                "intent": "service"
            }),
            "/tmp",
        )
        .expect("start retargetable service");
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
    assert!(deps_text.contains("shell:bg-2 -> capability:@api.http"));

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
    assert!(guidance_text.contains(
        "background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":[\"@api.http\"]}"
    ));
    assert!(guidance_text.contains(
        "orchestration_list_dependencies {\"filter\":\"missing\",\"capability\":\"@api.http\"}"
    ));

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
        "background_shell_update_service {\"jobId\":\"bg-1\",\"capabilities\":[\"@api.http\"]}"
    ));
    assert!(actions_text.contains(
        "background_shell_update_dependencies {\"jobId\":\"bg-2\",\"dependsOnCapabilities\":[\"@other.role\"]}"
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
