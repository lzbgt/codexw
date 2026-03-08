use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use serde_json::json;

#[cfg(test)]
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellOrigin;
use crate::orchestration_view::DependencyFilter;
use crate::orchestration_view::DependencySelection;
use crate::orchestration_view::WorkerFilter;
use crate::orchestration_view::orchestration_guidance_summary;
use crate::orchestration_view::orchestration_overview_summary;
use crate::orchestration_view::orchestration_runtime_summary;
use crate::orchestration_view::render_orchestration_actions_for_tool;
use crate::orchestration_view::render_orchestration_actions_for_tool_capability;
use crate::orchestration_view::render_orchestration_blockers_for_capability;
use crate::orchestration_view::render_orchestration_dependencies;
use crate::orchestration_view::render_orchestration_guidance_for_capability;
use crate::orchestration_view::render_orchestration_workers;
use crate::orchestration_view::render_orchestration_workers_with_filter;
use crate::state::AppState;

#[cfg(test)]
#[path = "client_dynamic_tools_tests.rs"]
mod tests;

const MAX_FILE_BYTES: u64 = 256 * 1024;
const DEFAULT_LIMIT: usize = 20;
const MAX_RESULTS: usize = 100;
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", ".next", "dist", "build"];

pub(crate) fn dynamic_tool_specs() -> Value {
    Value::Array(vec![
        json!({
            "name": "orchestration_status",
            "description": "Summarize the current orchestration state, including worker counts, dependency health, and the highest-priority next action when one exists.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "orchestration_list_workers",
            "description": "Render the current orchestration worker graph, optionally filtered to all, blockers, dependencies, agents, shells, services, capabilities, terminals, guidance, or actions. Blockers, guidance, and actions may also be narrowed to one @capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "enum": ["all", "blockers", "dependencies", "agents", "shells", "services", "capabilities", "terminals", "guidance", "actions"]
                    },
                    "capability": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "orchestration_suggest_actions",
            "description": "Render concrete next-step dynamic tool suggestions for the current orchestration state, such as capability inspection, readiness waits, service attach, or scoped cleanup actions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "capability": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "orchestration_list_dependencies",
            "description": "Render the current orchestration dependency graph, optionally filtered to all, blocking, sidecars, missing, booting, ambiguous, or satisfied dependency states and optionally narrowed to one @capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "enum": ["all", "blocking", "sidecars", "missing", "booting", "ambiguous", "satisfied"]
                    },
                    "capability": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "workspace_list_dir",
            "description": "List files and directories under a workspace directory. Defaults to the workspace root.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_RESULTS}
                }
            }
        }),
        json!({
            "name": "workspace_stat_path",
            "description": "Inspect a workspace path and report whether it is a file or directory, plus basic metadata.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "workspace_read_file",
            "description": "Read a UTF-8 text file from the current workspace. Supports optional 1-based startLine and endLine filters.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "startLine": {"type": "integer", "minimum": 1},
                    "endLine": {"type": "integer", "minimum": 1}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "workspace_find_files",
            "description": "Find workspace file paths whose relative path contains the given query substring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_RESULTS}
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "workspace_search_text",
            "description": "Search UTF-8 text files in the current workspace for lines containing the given query substring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_RESULTS}
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "background_shell_start",
            "description": "Start a long-running shell command in the background so you can continue other work in the same turn. Use `intent=prerequisite` for critical-path work you will need before finishing, `intent=observation` for non-blocking sidecar work such as tests or searches, and `intent=service` for reusable long-lived helpers such as dev servers. Jobs may also declare `dependsOnCapabilities` so the orchestration graph can model durable dependencies on reusable services, and service jobs may additionally declare `capabilities`, `readyPattern`, `protocol`, `endpoint`, `attachHint`, and structured `recipes` so the wrapper can distinguish booting versus ready services, expose a reusable attach surface, and invoke typed service recipes later.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string"},
                    "cwd": {"type": "string"},
                    "intent": {
                        "type": "string",
                        "enum": ["prerequisite", "observation", "service"]
                    },
                    "label": {"type": "string"},
                    "capabilities": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "dependsOnCapabilities": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "readyPattern": {"type": "string"},
                    "protocol": {"type": "string"},
                    "endpoint": {"type": "string"},
                    "attachHint": {"type": "string"},
                    "recipes": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "description": {"type": "string"},
                                "example": {"type": "string"},
                                "parameters": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"},
                                            "description": {"type": "string"},
                                            "default": {"type": "string"},
                                            "required": {"type": "boolean"}
                                        },
                                        "required": ["name"]
                                    }
                                },
                                "action": {
                                    "type": "object",
                                    "properties": {
                                        "type": {
                                            "type": "string",
                                            "enum": ["informational", "stdin", "http", "tcp", "redis"]
                                        },
                                        "text": {"type": "string"},
                                        "appendNewline": {"type": "boolean"},
                                        "method": {"type": "string"},
                                        "path": {"type": "string"},
                                        "body": {"type": "string"},
                                        "payload": {"type": "string"},
                                        "command": {
                                            "type": "array",
                                            "items": {"type": "string"}
                                        },
                                        "expectSubstring": {"type": "string"},
                                        "readTimeoutMs": {"type": "integer", "minimum": 1},
                                        "headers": {
                                            "type": "object",
                                            "additionalProperties": {"type": "string"}
                                        },
                                        "expectedStatus": {
                                            "type": "integer",
                                            "minimum": 100,
                                            "maximum": 599
                                        }
                                    }
                                }
                            },
                            "required": ["name"]
                        }
                    }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "background_shell_poll",
            "description": "Inspect a background shell job by jobId, alias, or @capability and fetch new output lines since an optional afterLine cursor.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "afterLine": {"type": "integer", "minimum": 0},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 200}
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "background_shell_send",
            "description": "Send stdin text to a running background shell job by jobId, alias, or @capability. Defaults to appending a trailing newline.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "text": {"type": "string"},
                    "appendNewline": {"type": "boolean"}
                },
                "required": ["jobId", "text"]
            }
        }),
        json!({
            "name": "background_shell_list_capabilities",
            "description": "List the reusable service capability registry, optionally filtered to healthy, missing, booting, or ambiguous capability states.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["all", "healthy", "missing", "booting", "ambiguous"]
                    }
                }
            }
        }),
        json!({
            "name": "background_shell_list_services",
            "description": "List reusable service shell jobs, optionally filtered to ready, booting, untracked, or conflicting services and optionally narrowed to one @capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["all", "ready", "booting", "untracked", "conflicts"]
                    },
                    "capability": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "background_shell_inspect_capability",
            "description": "Inspect one reusable service capability and show its current providers, provider metadata, and consumers. Accepts `capability` with or without the leading @.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "capability": {"type": "string"}
                },
                "required": ["capability"]
            }
        }),
        json!({
            "name": "background_shell_attach",
            "description": "Show structured attachment metadata for a service background shell job by jobId, alias, or @capability, including endpoint, capabilities, and attach hints when declared.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"}
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "background_shell_wait_ready",
            "description": "Wait for a service background shell job with a declared readyPattern to become ready. Supports jobId, alias, or @capability references.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "timeoutMs": {"type": "integer", "minimum": 0}
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "background_shell_invoke_recipe",
            "description": "Invoke a structured recipe declared by a service background shell job. Supports jobId, alias, or @capability references.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"},
                    "recipe": {"type": "string"},
                    "waitForReadyMs": {"type": "integer", "minimum": 0},
                    "args": {
                        "type": "object",
                        "additionalProperties": {
                            "type": ["string", "number", "boolean"]
                        }
                    }
                },
                "required": ["jobId", "recipe"]
            }
        }),
        json!({
            "name": "background_shell_list",
            "description": "List wrapper-owned background shell jobs with their current status.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "background_shell_terminate",
            "description": "Terminate a running background shell job by jobId, alias, or @capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": {"type": "string"}
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "background_shell_clean",
            "description": "Terminate local background shell jobs by scope. Supports all, blockers, shells, or services. Blocker cleanup can optionally target one @capability to clear only prerequisite shells gated on that reusable role, and service cleanup can optionally target one @capability to resolve ambiguous reusable roles.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "scope": {
                        "type": "string",
                        "enum": ["all", "blockers", "shells", "services"]
                    },
                    "capability": {"type": "string"}
                }
            }
        }),
    ])
}

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
        "background_shell_list_capabilities" => state
            .orchestration
            .background_shells
            .list_capabilities_from_tool(arguments),
        "background_shell_list_services" => state
            .orchestration
            .background_shells
            .list_services_from_tool(arguments),
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
    if let Some(guidance) = orchestration_guidance_summary(state) {
        lines.push(format!("next action     {guidance}"));
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
            Some(capability) => render_orchestration_guidance_for_capability(state, capability)?,
            None => render_orchestration_workers_with_filter(state, filter),
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

fn workspace_list_dir(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_list_dir expects an object argument".to_string())?;
    let root = workspace_root(resolved_cwd)?;
    let target = object
        .get("path")
        .and_then(Value::as_str)
        .map(|path| resolve_workspace_path(&root, path))
        .transpose()?
        .unwrap_or_else(|| root.clone());
    let metadata = fs::metadata(&target)
        .map_err(|err| format!("failed to stat `{}`: {err}", target.display()))?;
    if !metadata.is_dir() {
        return Err(format!("`{}` is not a directory", target.display()));
    }
    let limit = extract_limit(object.get("limit"));
    let mut entries = fs::read_dir(&target)
        .map_err(|err| format!("failed to read directory `{}`: {err}", target.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read directory `{}`: {err}", target.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    let total_entries = entries.len();

    let relative = rel_display(&root, &target);
    let mut rendered = vec![format!("Directory: {}", normalize_root_label(&relative))];
    if entries.is_empty() {
        rendered.push("(empty directory)".to_string());
        return Ok(rendered.join("\n"));
    }
    for entry in entries.into_iter().take(limit) {
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|err| format!("failed to stat `{}`: {err}", path.display()))?;
        let kind = if metadata.is_dir() { "dir " } else { "file" };
        let size = if metadata.is_file() {
            format!("{} bytes", metadata.len())
        } else {
            "-".to_string()
        };
        rendered.push(format!("{kind}  {:<8} {}", size, rel_display(&root, &path)));
    }
    if rendered.len() == limit + 1 && total_entries > limit {
        rendered.push(format!(
            "... {} more entries omitted",
            total_entries - limit
        ));
    }
    Ok(rendered.join("\n"))
}

fn workspace_stat_path(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_stat_path expects an object argument".to_string())?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspace_stat_path requires `path`".to_string())?;
    let root = workspace_root(resolved_cwd)?;
    let target = resolve_workspace_path(&root, path)?;
    let metadata = fs::metadata(&target)
        .map_err(|err| format!("failed to stat `{}`: {err}", target.display()))?;
    let relative = rel_display(&root, &target);
    let kind = if metadata.is_dir() {
        "directory"
    } else if metadata.is_file() {
        "file"
    } else {
        "other"
    };
    let mut rendered = vec![
        format!("Path: {relative}"),
        format!("Type: {kind}"),
        format!("Size: {} bytes", metadata.len()),
    ];
    if let Ok(modified) = metadata.modified()
        && let Ok(value) = modified.duration_since(std::time::UNIX_EPOCH)
    {
        rendered.push(format!("Modified: {}", value.as_secs()));
    }
    Ok(rendered.join("\n"))
}

fn workspace_read_file(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_read_file expects an object argument".to_string())?;
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspace_read_file requires `path`".to_string())?;
    let root = workspace_root(resolved_cwd)?;
    let file_path = resolve_workspace_path(&root, path)?;
    let metadata = fs::metadata(&file_path)
        .map_err(|err| format!("failed to stat `{}`: {err}", file_path.display()))?;
    if !metadata.is_file() {
        return Err(format!("`{}` is not a regular file", file_path.display()));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!(
            "`{}` is too large to read safely ({} bytes)",
            file_path.display(),
            metadata.len()
        ));
    }

    let contents = fs::read_to_string(&file_path).map_err(|err| {
        format!(
            "failed to read UTF-8 text from `{}`: {err}",
            file_path.display()
        )
    })?;
    let start_line = object
        .get("startLine")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1);
    let end_line = object
        .get("endLine")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    let relative = rel_display(&root, &file_path);

    let lines = contents.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return Ok(format!("File: {relative}\n(empty file)"));
    }
    let bounded_start = start_line.max(1).min(lines.len());
    let bounded_end = end_line
        .unwrap_or(lines.len())
        .max(bounded_start)
        .min(lines.len());
    let mut rendered = vec![format!("File: {relative}")];
    for (index, line) in lines
        .iter()
        .enumerate()
        .skip(bounded_start - 1)
        .take(bounded_end - bounded_start + 1)
    {
        rendered.push(format!("{:>4} | {}", index + 1, line));
    }
    Ok(rendered.join("\n"))
}

fn workspace_find_files(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_find_files expects an object argument".to_string())?;
    let query = object
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "workspace_find_files requires a non-empty `query`".to_string())?;
    let limit = extract_limit(object.get("limit"));
    let root = workspace_root(resolved_cwd)?;
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();
    walk_workspace(&root, &mut |path| {
        let relative = rel_display(&root, path);
        if relative.to_lowercase().contains(&query_lower) {
            matches.push(relative);
        }
        matches.len() < limit
    })?;

    if matches.is_empty() {
        return Ok(format!("No workspace files matched `{query}`."));
    }

    let mut rendered = vec![format!("File matches for `{query}`:")];
    rendered.extend(
        matches
            .iter()
            .enumerate()
            .map(|(index, path)| format!("{:>2}. {path}", index + 1)),
    );
    Ok(rendered.join("\n"))
}

fn workspace_search_text(arguments: &Value, resolved_cwd: &str) -> Result<String, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "workspace_search_text expects an object argument".to_string())?;
    let query = object
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "workspace_search_text requires a non-empty `query`".to_string())?;
    let limit = extract_limit(object.get("limit"));
    let root = workspace_root(resolved_cwd)?;
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();
    walk_workspace(&root, &mut |path| {
        if matches.len() >= limit {
            return false;
        }
        let Ok(metadata) = fs::metadata(path) else {
            return true;
        };
        if metadata.len() > MAX_FILE_BYTES {
            return true;
        }
        let Ok(contents) = fs::read_to_string(path) else {
            return true;
        };
        for (line_index, line) in contents.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                matches.push(format!(
                    "{}:{}: {}",
                    rel_display(&root, path),
                    line_index + 1,
                    line
                ));
                if matches.len() >= limit {
                    break;
                }
            }
        }
        true
    })?;

    if matches.is_empty() {
        return Ok(format!("No text matches for `{query}` in the workspace."));
    }

    let mut rendered = vec![format!("Text matches for `{query}`:")];
    rendered.extend(matches);
    Ok(rendered.join("\n"))
}

fn workspace_root(resolved_cwd: &str) -> Result<PathBuf, String> {
    let root = Path::new(resolved_cwd);
    fs::canonicalize(root)
        .map_err(|err| format!("failed to resolve workspace root `{resolved_cwd}`: {err}"))
}

fn resolve_workspace_path(root: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let candidate = if Path::new(raw_path).is_absolute() {
        PathBuf::from(raw_path)
    } else {
        root.join(raw_path)
    };
    let resolved = fs::canonicalize(&candidate)
        .map_err(|err| format!("failed to resolve `{}`: {err}", candidate.display()))?;
    if !resolved.starts_with(root) {
        return Err(format!(
            "`{}` is outside the current workspace",
            resolved.display()
        ));
    }
    Ok(resolved)
}

fn walk_workspace(root: &Path, visit: &mut impl FnMut(&Path) -> bool) -> Result<(), String> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir)
            .map_err(|err| format!("failed to read directory `{}`: {err}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|err| {
                format!(
                    "failed to read directory entry in `{}`: {err}",
                    dir.display()
                )
            })?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|err| format!("failed to stat `{}`: {err}", path.display()))?;
            if file_type.is_dir() {
                if should_skip_dir(&path) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if file_type.is_file() && !visit(&path) {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| SKIP_DIRS.iter().any(|skip| skip == &name))
}

fn rel_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn normalize_root_label(relative: &str) -> &str {
    if relative.is_empty() { "." } else { relative }
}

fn extract_limit(limit: Option<&Value>) -> usize {
    limit
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .map(|value| value.clamp(1, MAX_RESULTS))
        .unwrap_or(DEFAULT_LIMIT)
}
