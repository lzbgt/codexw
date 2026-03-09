use serde_json::Value;

use crate::state::AppState;

pub(crate) fn execute_background_shell_service_tool(
    tool: &str,
    arguments: &Value,
    state: &AppState,
) -> Result<String, String> {
    match tool {
        "background_shell_list_capabilities" => state
            .orchestration
            .background_shells
            .list_capabilities_from_tool(arguments),
        "background_shell_list_services" => state
            .orchestration
            .background_shells
            .list_services_from_tool(arguments),
        "background_shell_update_service" => state
            .orchestration
            .background_shells
            .update_service_from_tool(arguments),
        "background_shell_update_dependencies" => state
            .orchestration
            .background_shells
            .update_dependencies_from_tool(arguments),
        "background_shell_inspect_capability" => state
            .orchestration
            .background_shells
            .inspect_capability_from_tool(arguments),
        "background_shell_attach" => state
            .orchestration
            .background_shells
            .attach_from_tool(arguments),
        "background_shell_wait_ready" => state
            .orchestration
            .background_shells
            .wait_ready_from_tool(arguments),
        "background_shell_invoke_recipe" => state
            .orchestration
            .background_shells
            .invoke_recipe_from_tool(arguments),
        _ => Err(format!(
            "unsupported background shell service tool `{tool}`"
        )),
    }
}
