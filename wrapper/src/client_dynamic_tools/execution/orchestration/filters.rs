use serde_json::Value;

use crate::orchestration_view::DependencyFilter;
use crate::orchestration_view::DependencySelection;
use crate::orchestration_view::WorkerFilter;
use crate::orchestration_view::render_orchestration_actions_for_tool;
use crate::orchestration_view::render_orchestration_actions_for_tool_capability;
use crate::orchestration_view::render_orchestration_blockers_for_capability;
use crate::orchestration_view::render_orchestration_dependencies;
use crate::orchestration_view::render_orchestration_guidance_for_tool;
use crate::orchestration_view::render_orchestration_guidance_for_tool_capability;
use crate::orchestration_view::render_orchestration_workers;
use crate::orchestration_view::render_orchestration_workers_with_filter;
use crate::state::AppState;

pub(crate) fn render_orchestration_workers_for_tool(
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

pub(crate) fn render_orchestration_actions_for_tool_from_args(
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

pub(crate) fn render_orchestration_dependencies_for_tool(
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
