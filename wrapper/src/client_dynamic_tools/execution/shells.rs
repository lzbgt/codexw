use serde_json::Value;

use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellOrigin;
use crate::state::AppState;

#[path = "shells/jobs.rs"]
mod jobs;
#[path = "shells/services.rs"]
mod services;

pub(crate) fn is_background_shell_tool(tool: &str) -> bool {
    matches!(
        tool,
        "background_shell_start"
            | "background_shell_list"
            | "background_shell_poll"
            | "background_shell_send"
            | "background_shell_set_alias"
            | "background_shell_terminate"
            | "background_shell_clean"
            | "background_shell_list_capabilities"
            | "background_shell_list_services"
            | "background_shell_update_service"
            | "background_shell_update_dependencies"
            | "background_shell_inspect_capability"
            | "background_shell_attach"
            | "background_shell_wait_ready"
            | "background_shell_invoke_recipe"
    )
}

pub(crate) fn execute_background_shell_tool_with_manager(
    tool: &str,
    params: &Value,
    arguments: &Value,
    resolved_cwd: &str,
    background_shells: &BackgroundShellManager,
) -> Result<String, String> {
    match tool {
        "background_shell_start"
        | "background_shell_list"
        | "background_shell_poll"
        | "background_shell_send"
        | "background_shell_set_alias"
        | "background_shell_terminate"
        | "background_shell_clean" => jobs::execute_background_shell_job_tool_with_manager(
            tool,
            params,
            arguments,
            resolved_cwd,
            background_shells,
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
            services::execute_background_shell_service_tool_with_manager(
                tool,
                arguments,
                background_shells,
            )
        }
        _ => Err(format!("unsupported client dynamic tool `{tool}`")),
    }
}

pub(crate) fn execute_background_shell_tool(
    tool: &str,
    params: &Value,
    arguments: &Value,
    resolved_cwd: &str,
    state: &AppState,
) -> Result<String, String> {
    execute_background_shell_tool_with_manager(
        tool,
        params,
        arguments,
        resolved_cwd,
        &state.orchestration.background_shells,
    )
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
