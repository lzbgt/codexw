use super::super::super::*;

#[test]
fn background_shell_clean_can_resolve_ambiguous_service_capability_conflicts() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api a",
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
                "label": "api b",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second provider");
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "db",
                "capabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start unrelated provider");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "background_shell_clean",
            "arguments": {
                "scope": "services",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("cleanup text");
    assert!(text.contains("Terminated 2"));
    assert!(text.contains("@api.http"));

    let remaining = state
        .orchestration
        .background_shells
        .render_service_shells_for_ps_filtered(None, None)
        .expect("render remaining services")
        .join("\n");
    assert!(!remaining.contains("api a"));
    assert!(!remaining.contains("api b"));
    assert!(remaining.contains("db"));

    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn background_shell_clean_can_target_blockers_by_capability() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
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
                "label": "db blocker",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db blocker");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "background_shell_clean",
            "arguments": {
                "scope": "blockers",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("cleanup text");
    assert!(text.contains("Terminated 1"));
    assert!(text.contains("@api.http"));

    let remaining = state
        .orchestration
        .background_shells
        .capability_dependency_summaries()
        .into_iter()
        .map(|summary| format!("{} -> {}", summary.job_id, summary.capability))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!remaining.contains("api.http"));
    assert!(remaining.contains("db.redis"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn background_shell_clean_rejects_capability_outside_service_scope() {
    let state = AppState::new(true, false);
    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "background_shell_clean",
            "arguments": {
                "scope": "shells",
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
            .contains("only valid with `scope=blockers` or `scope=services`")
    );
}
