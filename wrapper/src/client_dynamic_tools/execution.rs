use serde_json::Value;
use serde_json::json;

#[cfg(test)]
use crate::background_shells::BackgroundShellManager;
use crate::client_dynamic_tools::workspace::workspace_find_files;
use crate::client_dynamic_tools::workspace::workspace_list_dir;
use crate::client_dynamic_tools::workspace::workspace_read_file;
use crate::client_dynamic_tools::workspace::workspace_search_text;
use crate::client_dynamic_tools::workspace::workspace_stat_path;
use crate::state::AppState;

#[path = "execution/orchestration.rs"]
mod orchestration;
#[path = "execution/shells.rs"]
mod shells;

#[cfg(test)]
pub(crate) fn execute_dynamic_tool_call(
    params: &Value,
    resolved_cwd: &str,
    background_shells: &BackgroundShellManager,
) -> Value {
    let mut state = AppState::new(true, false);
    state.orchestration.background_shells = background_shells.clone();
    execute_dynamic_tool_call_with_state(params, resolved_cwd, &state)
}

pub(crate) fn execute_dynamic_tool_call_with_state(
    params: &Value,
    resolved_cwd: &str,
    state: &AppState,
) -> Value {
    let tool = params
        .get("tool")
        .and_then(Value::as_str)
        .unwrap_or("dynamic tool");
    let arguments = params.get("arguments").unwrap_or(&Value::Null);
    let result = match tool {
        "orchestration_status" => Ok(orchestration::render_orchestration_status_for_tool(state)),
        "orchestration_list_workers" => {
            orchestration::render_orchestration_workers_for_tool(arguments, state)
        }
        "orchestration_suggest_actions" => {
            orchestration::render_orchestration_actions_for_tool_from_args(arguments, state)
        }
        "orchestration_list_dependencies" => {
            orchestration::render_orchestration_dependencies_for_tool(arguments, state)
        }
        // Retained for already-running older sessions that were given the
        // previous workspace tool bundle before new threads stopped
        // advertising it.
        "workspace_list_dir" => workspace_list_dir(arguments, resolved_cwd),
        "workspace_stat_path" => workspace_stat_path(arguments, resolved_cwd),
        "workspace_read_file" => workspace_read_file(arguments, resolved_cwd),
        "workspace_find_files" => workspace_find_files(arguments, resolved_cwd),
        "workspace_search_text" => workspace_search_text(arguments, resolved_cwd),
        _ => shells::execute_background_shell_tool(tool, params, arguments, resolved_cwd, state),
    };

    match result {
        Ok(text) => json!({
            "contentItems": [{"type": "inputText", "text": text}],
            "success": true
        }),
        Err(err) => json!({
            "contentItems": [{"type": "inputText", "text": err}],
            "success": false
        }),
    }
}
