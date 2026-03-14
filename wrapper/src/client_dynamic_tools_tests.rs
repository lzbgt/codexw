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
fn workspace_tool_specs_keep_bounded_read_only_framing() {
    let specs = dynamic_tool_specs().as_array().expect("array").clone();
    let tool_description = |name: &str| {
        specs
            .iter()
            .find(|tool| tool.get("name").and_then(serde_json::Value::as_str) == Some(name))
            .and_then(|tool| tool.get("description").and_then(serde_json::Value::as_str))
            .unwrap_or_else(|| panic!("missing description for {}", name))
    };

    assert!(
        tool_description("workspace_list_dir").contains("bounded")
            && tool_description("workspace_list_dir").contains("read-only inspection")
    );
    assert!(tool_description("workspace_stat_path").contains("read-only metadata"));
    assert!(tool_description("workspace_read_file").contains("bounded read-only inspection"));
    assert!(tool_description("workspace_find_files").contains("bounded set"));
    assert!(tool_description("workspace_search_text").contains("bounded set"));
}
