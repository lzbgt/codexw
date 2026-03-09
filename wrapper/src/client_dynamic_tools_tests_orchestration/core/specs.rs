use super::super::super::*;

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
