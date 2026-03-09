use super::super::super::*;

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
