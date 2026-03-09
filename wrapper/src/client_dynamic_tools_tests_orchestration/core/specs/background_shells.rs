use super::super::super::super::*;

#[test]
fn dynamic_tool_specs_include_background_shell_job_and_service_groups() {
    let specs = dynamic_tool_specs();
    let names = specs
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();

    for expected in [
        "background_shell_start",
        "background_shell_poll",
        "background_shell_send",
        "background_shell_set_alias",
        "background_shell_list",
        "background_shell_terminate",
        "background_shell_clean",
    ] {
        assert!(names.contains(&expected), "missing job tool {expected}");
    }

    for expected in [
        "background_shell_list_capabilities",
        "background_shell_list_services",
        "background_shell_update_service",
        "background_shell_update_dependencies",
        "background_shell_inspect_capability",
        "background_shell_attach",
        "background_shell_wait_ready",
        "background_shell_invoke_recipe",
    ] {
        assert!(names.contains(&expected), "missing service tool {expected}");
    }
}
