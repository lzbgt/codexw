use serde_json::Value;

use crate::background_shells::BackgroundShellOrigin;
use crate::state::AppState;

pub(crate) fn execute_background_shell_job_tool(
    tool: &str,
    _params: &Value,
    arguments: &Value,
    resolved_cwd: &str,
    state: &AppState,
    origin: BackgroundShellOrigin,
) -> Result<String, String> {
    match tool {
        "background_shell_start" => state
            .orchestration
            .background_shells
            .start_from_tool_with_context(arguments, resolved_cwd, origin),
        "background_shell_list" => Ok(state.orchestration.background_shells.list_from_tool()),
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
        "background_shell_terminate" => state
            .orchestration
            .background_shells
            .terminate_from_tool(arguments),
        "background_shell_clean" => state
            .orchestration
            .background_shells
            .clean_from_tool(arguments),
        _ => Err(format!("unsupported background shell job tool `{tool}`")),
    }
}
