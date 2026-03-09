use serde_json::Value;
use serde_json::json;

#[cfg(test)]
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellOrigin;
use crate::client_dynamic_tools::workspace::workspace_find_files;
use crate::client_dynamic_tools::workspace::workspace_list_dir;
use crate::client_dynamic_tools::workspace::workspace_read_file;
use crate::client_dynamic_tools::workspace::workspace_search_text;
use crate::client_dynamic_tools::workspace::workspace_stat_path;
use crate::orchestration_view::DependencyFilter;
use crate::orchestration_view::DependencySelection;
use crate::orchestration_view::WorkerFilter;
use crate::orchestration_view::orchestration_next_action_summary_for_tool;
use crate::orchestration_view::orchestration_overview_summary;
use crate::orchestration_view::orchestration_runtime_summary;
use crate::orchestration_view::render_orchestration_actions_for_tool;
use crate::orchestration_view::render_orchestration_actions_for_tool_capability;
use crate::orchestration_view::render_orchestration_blockers_for_capability;
use crate::orchestration_view::render_orchestration_dependencies;
use crate::orchestration_view::render_orchestration_guidance_for_tool;
use crate::orchestration_view::render_orchestration_guidance_for_tool_capability;
use crate::orchestration_view::render_orchestration_workers;
use crate::orchestration_view::render_orchestration_workers_with_filter;
use crate::state::AppState;

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
        "orchestration_status" => Ok(render_orchestration_status_for_tool(state)),
        "orchestration_list_workers" => render_orchestration_workers_for_tool(arguments, state),
        "orchestration_suggest_actions" => {
            render_orchestration_actions_for_tool_from_args(arguments, state)
        }
        "orchestration_list_dependencies" => {
            render_orchestration_dependencies_for_tool(arguments, state)
        }
        "workspace_list_dir" => workspace_list_dir(arguments, resolved_cwd),
        "workspace_stat_path" => workspace_stat_path(arguments, resolved_cwd),
        "workspace_read_file" => workspace_read_file(arguments, resolved_cwd),
        "workspace_find_files" => workspace_find_files(arguments, resolved_cwd),
        "workspace_search_text" => workspace_search_text(arguments, resolved_cwd),
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

fn render_orchestration_status_for_tool(state: &AppState) -> String {
    let mut lines = vec![format!(
        "orchestration   {}",
        orchestration_overview_summary(state)
    )];
    if let Some(runtime) = orchestration_runtime_summary(state) {
        lines.push(format!("runtime         {runtime}"));
    }
    if let Some(next_action) = orchestration_next_action_summary_for_tool(state) {
        lines.push(format!("next action     {next_action}"));
    }
    lines.join("\n")
}

fn render_orchestration_workers_for_tool(
    arguments: &Value,
    state: &AppState,
) -> Result<String, String> {
    let object = arguments.as_object();
    let filter = parse_worker_filter_for_tool(
        object
            .and_then(|object| object.get("filter"))
            .and_then(Value::as_str),
    )?;
    let capability = parse_optional_capability_for_tool(
        object
            .and_then(|object| object.get("capability"))
            .and_then(Value::as_str),
        "orchestration_list_workers",
    )?;
    Ok(if matches!(filter, WorkerFilter::All) {
        if capability.is_some() {
            return Err(
                "orchestration_list_workers `capability` is only supported with `filter=blockers`, `filter=guidance`, or `filter=actions`"
                    .to_string(),
            );
        }
        render_orchestration_workers(state)
    } else if matches!(filter, WorkerFilter::Blockers) {
        match capability.as_deref() {
            Some(capability) => render_orchestration_blockers_for_capability(state, capability)?,
            None => render_orchestration_workers_with_filter(state, filter),
        }
    } else if matches!(filter, WorkerFilter::Guidance) {
        match capability.as_deref() {
            Some(capability) => {
                render_orchestration_guidance_for_tool_capability(state, capability)?
            }
            None => render_orchestration_guidance_for_tool(state),
        }
    } else if matches!(filter, WorkerFilter::Actions) {
        match capability.as_deref() {
            Some(capability) => {
                render_orchestration_actions_for_tool_capability(state, capability)?
            }
            None => render_orchestration_actions_for_tool(state),
        }
    } else {
        if capability.is_some() {
            return Err(
                "orchestration_list_workers `capability` is only supported with `filter=blockers`, `filter=guidance`, or `filter=actions`"
                    .to_string(),
            );
        }
        render_orchestration_workers_with_filter(state, filter)
    })
}

fn render_orchestration_actions_for_tool_from_args(
    arguments: &Value,
    state: &AppState,
) -> Result<String, String> {
    let capability = parse_optional_capability_for_tool(
        arguments
            .as_object()
            .and_then(|object| object.get("capability"))
            .and_then(Value::as_str),
        "orchestration_suggest_actions",
    )?;
    match capability.as_deref() {
        Some(capability) => render_orchestration_actions_for_tool_capability(state, capability),
        None => Ok(render_orchestration_actions_for_tool(state)),
    }
}

fn render_orchestration_dependencies_for_tool(
    arguments: &Value,
    state: &AppState,
) -> Result<String, String> {
    let object = arguments.as_object();
    let filter = parse_dependency_filter_for_tool(
        object
            .and_then(|object| object.get("filter"))
            .and_then(Value::as_str),
    )?;
    let capability = parse_dependency_capability_for_tool(
        object
            .and_then(|object| object.get("capability"))
            .and_then(Value::as_str),
    )?;
    Ok(render_orchestration_dependencies(
        state,
        &DependencySelection { filter, capability },
    ))
}

fn parse_worker_filter_for_tool(raw: Option<&str>) -> Result<WorkerFilter, String> {
    match raw.unwrap_or("all") {
        "all" => Ok(WorkerFilter::All),
        "blockers" | "blocking" | "prereqs" => Ok(WorkerFilter::Blockers),
        "dependencies" | "deps" => Ok(WorkerFilter::Dependencies),
        "agents" => Ok(WorkerFilter::Agents),
        "shells" => Ok(WorkerFilter::Shells),
        "services" => Ok(WorkerFilter::Services),
        "capabilities" | "caps" | "cap" => Ok(WorkerFilter::Capabilities),
        "terminals" => Ok(WorkerFilter::Terminals),
        "guidance" | "guide" | "next" => Ok(WorkerFilter::Guidance),
        "actions" | "action" | "suggest" | "suggestions" => Ok(WorkerFilter::Actions),
        other => Err(format!(
            "orchestration_list_workers `filter` must be one of `all`, `blockers`, `dependencies`, `agents`, `shells`, `services`, `capabilities`, `terminals`, `guidance`, or `actions`, got `{other}`"
        )),
    }
}

fn parse_dependency_filter_for_tool(raw: Option<&str>) -> Result<DependencyFilter, String> {
    match raw.unwrap_or("all") {
        "all" => Ok(DependencyFilter::All),
        "blocking" | "blockers" => Ok(DependencyFilter::Blocking),
        "sidecars" | "sidecar" => Ok(DependencyFilter::Sidecars),
        "missing" => Ok(DependencyFilter::Missing),
        "booting" => Ok(DependencyFilter::Booting),
        "ambiguous" | "conflicts" | "conflict" => Ok(DependencyFilter::Ambiguous),
        "satisfied" | "ready" => Ok(DependencyFilter::Satisfied),
        other => Err(format!(
            "orchestration_list_dependencies `filter` must be one of `all`, `blocking`, `sidecars`, `missing`, `booting`, `ambiguous`, or `satisfied`, got `{other}`"
        )),
    }
}

fn parse_dependency_capability_for_tool(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let normalized = raw.trim().trim_start_matches('@');
    if normalized.is_empty() {
        return Err(
            "orchestration_list_dependencies `capability` must be a non-empty capability name"
                .to_string(),
        );
    }
    Ok(Some(normalized.to_string()))
}

fn parse_optional_capability_for_tool(
    raw: Option<&str>,
    context: &str,
) -> Result<Option<String>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let normalized = raw.trim().trim_start_matches('@');
    if normalized.is_empty() {
        return Err(format!(
            "{context} `capability` must be a non-empty capability name"
        ));
    }
    Ok(Some(normalized.to_string()))
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
