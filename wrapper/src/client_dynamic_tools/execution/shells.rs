use serde_json::Value;

use crate::background_shells::BackgroundShellOrigin;
use crate::state::AppState;

pub(crate) fn execute_background_shell_tool(
    tool: &str,
    params: &Value,
    arguments: &Value,
    resolved_cwd: &str,
    state: &AppState,
) -> Result<String, String> {
    match tool {
        "background_shell_start" => state
            .orchestration
            .background_shells
            .start_from_tool_with_context(arguments, resolved_cwd, dynamic_tool_origin(params)),
        "background_shell_poll" => state
            .orchestration
            .background_shells
            .poll_from_tool(arguments),
        "background_shell_send" => state
            .orchestration
            .background_shells
            .send_input_from_tool(arguments),
        "background_shell_set_alias" => state
            .orchestration
            .background_shells
            .update_alias_from_tool(arguments),
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
        "background_shell_list" => Ok(state.orchestration.background_shells.list_from_tool()),
        "background_shell_terminate" => state
            .orchestration
            .background_shells
            .terminate_from_tool(arguments),
        "background_shell_clean" => state
            .orchestration
            .background_shells
            .clean_from_tool(arguments),
        _ => Err(format!("unsupported client dynamic tool `{tool}`")),
    }
}

fn dynamic_tool_origin(params: &Value) -> BackgroundShellOrigin {
    BackgroundShellOrigin {
        source_thread_id: params
            .get("threadId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        source_call_id: params
            .get("callId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        source_tool: params
            .get("tool")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    }
}
