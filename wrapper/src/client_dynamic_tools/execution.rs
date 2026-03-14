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

const LEGACY_WORKSPACE_TOOLS: &[&str] = &[
    "workspace_list_dir",
    "workspace_stat_path",
    "workspace_read_file",
    "workspace_find_files",
    "workspace_search_text",
];

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

#[cfg(test)]
pub(crate) fn legacy_workspace_tool_names() -> &'static [&'static str] {
    LEGACY_WORKSPACE_TOOLS
}

fn is_legacy_workspace_tool(tool: &str) -> bool {
    LEGACY_WORKSPACE_TOOLS.contains(&tool)
}

fn execute_legacy_workspace_tool(
    tool: &str,
    arguments: &Value,
    resolved_cwd: &str,
) -> Option<Result<String, String>> {
    Some(match tool {
        "workspace_list_dir" => workspace_list_dir(arguments, resolved_cwd),
        "workspace_stat_path" => workspace_stat_path(arguments, resolved_cwd),
        "workspace_read_file" => workspace_read_file(arguments, resolved_cwd),
        "workspace_find_files" => workspace_find_files(arguments, resolved_cwd),
        "workspace_search_text" => workspace_search_text(arguments, resolved_cwd),
        _ => return None,
    })
}

pub(crate) fn legacy_workspace_tool_notice(tool: &str) -> Option<String> {
    if is_legacy_workspace_tool(tool) {
        Some(format!(
            "[tool] legacy workspace compatibility path: {tool} is hidden on new threads; this call came from an older session"
        ))
    } else {
        None
    }
}

pub(crate) fn legacy_workspace_tool_failure_notice(tool: &str, result: &Value) -> Option<String> {
    let failure_text = result
        .get("success")
        .and_then(Value::as_bool)
        .filter(|success| !success)
        .and_then(|_| result.get("contentItems"))
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)?;
    is_legacy_workspace_tool(tool)
        .then_some(())
        .map(|_| format!("[tool] legacy workspace compatibility failure: {tool}: {failure_text}"))
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
    let result =
        execute_legacy_workspace_tool(tool, arguments, resolved_cwd).unwrap_or_else(|| {
            match tool {
                "orchestration_status" => {
                    Ok(orchestration::render_orchestration_status_for_tool(state))
                }
                "orchestration_list_workers" => {
                    orchestration::render_orchestration_workers_for_tool(arguments, state)
                }
                "orchestration_suggest_actions" => {
                    orchestration::render_orchestration_actions_for_tool_from_args(arguments, state)
                }
                "orchestration_list_dependencies" => {
                    orchestration::render_orchestration_dependencies_for_tool(arguments, state)
                }
                _ => {
                    // Retained only for already-running older sessions that were given the
                    // previous workspace tool bundle before new threads stopped
                    // advertising it. New threads should prefer host shell or Python for
                    // workspace inspection.
                    shells::execute_background_shell_tool(
                        tool,
                        params,
                        arguments,
                        resolved_cwd,
                        state,
                    )
                }
            }
        });

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
