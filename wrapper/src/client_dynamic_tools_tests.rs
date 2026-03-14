pub(super) use super::dynamic_tool_specs;
pub(super) use super::execute_dynamic_tool_call;
pub(super) use super::execute_dynamic_tool_call_with_state;
pub(super) use crate::state::AppState;
pub(super) use serde_json::json;

#[path = "client_dynamic_tools_tests_orchestration.rs"]
mod orchestration;
#[path = "client_dynamic_tools_tests_shells.rs"]
mod shells;
#[path = "client_dynamic_tools_tests_workspace.rs"]
mod workspace;

#[test]
fn dynamic_tool_specs_exclude_workspace_tools() {
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
fn workspace_tools_remain_executable_for_older_sessions_even_though_not_advertised() {
    let specs = dynamic_tool_specs();
    let names = specs
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    for legacy_tool in super::legacy_workspace_tool_names() {
        assert!(!names.contains(legacy_tool));
    }

    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("hello.txt"), "alpha\nbeta\n").expect("write");

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "workspace_read_file",
            "arguments": {"path": "hello.txt", "startLine": 2}
        }),
        workspace.path().to_str().expect("utf8 path"),
        &crate::background_shells::BackgroundShellManager::default(),
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("text output");
    assert!(text.contains("File: hello.txt"));
    assert!(text.contains("   2 | beta"));
}

#[test]
fn legacy_workspace_tool_names_match_the_hidden_compatibility_surface() {
    assert_eq!(
        super::legacy_workspace_tool_names(),
        &[
            "workspace_list_dir",
            "workspace_stat_path",
            "workspace_read_file",
            "workspace_find_files",
            "workspace_search_text",
        ]
    );
}

#[test]
fn legacy_workspace_tool_notice_is_limited_to_hidden_compatibility_tools() {
    let notice = super::legacy_workspace_tool_notice("workspace_read_file")
        .expect("legacy workspace notice");
    assert!(notice.contains("workspace_read_file"));
    assert!(notice.contains("hidden on new threads"));
    assert!(notice.contains("older session"));

    assert!(super::legacy_workspace_tool_notice("orchestration_status").is_none());
    assert!(super::legacy_workspace_tool_notice("background_shell_start").is_none());
}

#[test]
fn legacy_workspace_tool_failure_notice_includes_failure_text_only_for_hidden_tools() {
    let result = json!({
        "success": false,
        "contentItems": [{"type": "inputText", "text": "legacy workspace compatibility scan exceeded 2000 entries"}]
    });

    let notice = super::legacy_workspace_tool_failure_notice("workspace_search_text", &result)
        .expect("legacy workspace failure notice");
    assert!(notice.contains("workspace_search_text"));
    assert!(notice.contains("legacy workspace compatibility failure"));
    assert!(notice.contains("scan exceeded 2000 entries"));

    assert!(super::legacy_workspace_tool_failure_notice("orchestration_status", &result).is_none());
    assert!(
        super::legacy_workspace_tool_failure_notice(
            "workspace_search_text",
            &json!({"success": true, "contentItems": [{"type": "inputText", "text": "ok"}]})
        )
        .is_none()
    );
}
