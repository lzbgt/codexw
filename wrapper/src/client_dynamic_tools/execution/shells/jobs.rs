use serde_json::Value;

use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellOrigin;
pub(crate) fn execute_background_shell_job_tool_with_manager(
    tool: &str,
    _params: &Value,
    arguments: &Value,
    resolved_cwd: &str,
    background_shells: &BackgroundShellManager,
    origin: BackgroundShellOrigin,
) -> Result<String, String> {
    match tool {
        "background_shell_start" => {
            background_shells.start_from_tool_with_context(arguments, resolved_cwd, origin)
        }
        "background_shell_list" => Ok(background_shells.list_from_tool()),
        "background_shell_poll" => background_shells.poll_from_tool(arguments),
        "background_shell_send" => background_shells.send_input_from_tool(arguments),
        "background_shell_set_alias" => background_shells.update_alias_from_tool(arguments),
        "background_shell_terminate" => background_shells.terminate_from_tool(arguments),
        "background_shell_clean" => background_shells.clean_from_tool(arguments),
        _ => Err(format!("unsupported background shell job tool `{tool}`")),
    }
}
