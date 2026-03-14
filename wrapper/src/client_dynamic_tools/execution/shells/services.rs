use serde_json::Value;

use crate::background_shells::BackgroundShellManager;

pub(crate) fn execute_background_shell_service_tool_with_manager(
    tool: &str,
    arguments: &Value,
    background_shells: &BackgroundShellManager,
) -> Result<String, String> {
    match tool {
        "background_shell_list_capabilities" => {
            background_shells.list_capabilities_from_tool(arguments)
        }
        "background_shell_list_services" => background_shells.list_services_from_tool(arguments),
        "background_shell_update_service" => background_shells.update_service_from_tool(arguments),
        "background_shell_update_dependencies" => {
            background_shells.update_dependencies_from_tool(arguments)
        }
        "background_shell_inspect_capability" => {
            background_shells.inspect_capability_from_tool(arguments)
        }
        "background_shell_attach" => background_shells.attach_from_tool(arguments),
        "background_shell_wait_ready" => background_shells.wait_ready_from_tool(arguments),
        "background_shell_invoke_recipe" => background_shells.invoke_recipe_from_tool(arguments),
        _ => Err(format!(
            "unsupported background shell service tool `{tool}`"
        )),
    }
}
