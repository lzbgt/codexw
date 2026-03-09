use serde_json::Value;

use crate::background_shells::BackgroundShellOrigin;
use crate::state::AppState;

#[path = "shells/jobs.rs"]
mod jobs;
#[path = "shells/services.rs"]
mod services;

pub(crate) fn execute_background_shell_tool(
    tool: &str,
    params: &Value,
    arguments: &Value,
    resolved_cwd: &str,
    state: &AppState,
) -> Result<String, String> {
    match tool {
        "background_shell_start"
        | "background_shell_list"
        | "background_shell_poll"
        | "background_shell_send"
        | "background_shell_set_alias"
        | "background_shell_terminate"
        | "background_shell_clean" => jobs::execute_background_shell_job_tool(
            tool,
            params,
            arguments,
            resolved_cwd,
            state,
            dynamic_tool_origin(params),
        ),
        "background_shell_list_capabilities"
        | "background_shell_list_services"
        | "background_shell_update_service"
        | "background_shell_update_dependencies"
        | "background_shell_inspect_capability"
        | "background_shell_attach"
        | "background_shell_wait_ready"
        | "background_shell_invoke_recipe" => {
            services::execute_background_shell_service_tool(tool, arguments, state)
        }
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
