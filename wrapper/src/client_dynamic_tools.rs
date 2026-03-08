use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use serde_json::json;

#[cfg(test)]
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellOrigin;
use crate::orchestration_view::DependencyFilter;
use crate::orchestration_view::WorkerFilter;
use crate::orchestration_view::orchestration_guidance_summary;
use crate::orchestration_view::orchestration_overview_summary;
use crate::orchestration_view::orchestration_runtime_summary;
use crate::orchestration_view::render_orchestration_dependencies;
use crate::orchestration_view::render_orchestration_workers;
use crate::orchestration_view::render_orchestration_workers_with_filter;
use crate::state::AppState;

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
            "description": "Render the current orchestration worker graph, optionally filtered to all, blockers, dependencies, agents, shells, services, capabilities, terminals, or guidance.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "enum": ["all", "blockers", "dependencies", "agents", "shells", "services", "capabilities", "terminals", "guidance"]
                    }
                }
            }
        }),
        json!({
            "name": "orchestration_list_dependencies",
            "description": "Render the current orchestration dependency graph, optionally filtered to all, blocking, sidecars, missing, booting, ambiguous, or satisfied dependency states.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "enum": ["all", "blocking", "sidecars", "missing", "booting", "ambiguous", "satisfied"]
                    }
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
    Ok(if matches!(filter, WorkerFilter::All) {
        render_orchestration_workers(state)
    } else {
        render_orchestration_workers_with_filter(state, filter)
    })
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
    Ok(render_orchestration_dependencies(state, filter))
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
        other => Err(format!(
            "orchestration_list_workers `filter` must be one of `all`, `blockers`, `dependencies`, `agents`, `shells`, `services`, `capabilities`, `terminals`, or `guidance`, got `{other}`"
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

#[cfg(test)]
mod tests {
    use super::dynamic_tool_specs;
    use super::execute_dynamic_tool_call;
    use super::execute_dynamic_tool_call_with_state;
    use crate::background_shells::BackgroundShellManager;
    use crate::state::AppState;
    use serde_json::json;

    #[test]
    fn dynamic_tool_specs_include_workspace_tools() {
        let specs = dynamic_tool_specs();
        let names = specs
            .as_array()
            .expect("array")
            .iter()
            .filter_map(|tool| tool.get("name").and_then(serde_json::Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "orchestration_status",
                "orchestration_list_workers",
                "orchestration_list_dependencies",
                "workspace_list_dir",
                "workspace_stat_path",
                "workspace_read_file",
                "workspace_find_files",
                "workspace_search_text",
                "background_shell_start",
                "background_shell_poll",
                "background_shell_send",
                "background_shell_list_capabilities",
                "background_shell_inspect_capability",
                "background_shell_attach",
                "background_shell_wait_ready",
                "background_shell_invoke_recipe",
                "background_shell_list",
                "background_shell_terminate"
            ]
        );
    }

    #[test]
    fn orchestration_status_reports_worker_and_guidance_summary() {
        let mut state = AppState::new(true, false);
        state
            .orchestration
            .background_shells
            .start_from_tool(
                &json!({
                    "command": "sleep 0.4",
                    "intent": "prerequisite",
                    "dependsOnCapabilities": ["api.http"]
                }),
                "/tmp",
            )
            .expect("start dependent shell");

        let result = execute_dynamic_tool_call_with_state(
            &json!({
                "tool": "orchestration_status"
            }),
            "/tmp",
            &state,
        );

        assert_eq!(result["success"], true);
        let rendered = result["contentItems"][0]["text"]
            .as_str()
            .expect("status text");
        assert!(rendered.contains("orchestration   main=1"));
        assert!(rendered.contains("cap_deps_missing=1"));
        assert!(rendered.contains("next action"));
        assert!(rendered.contains("missing service capability @api.http"));
        let _ = state
            .orchestration
            .background_shells
            .terminate_all_running();
    }

    #[test]
    fn orchestration_list_workers_supports_filtered_capability_and_guidance_views() {
        let mut state = AppState::new(true, false);
        state
            .orchestration
            .background_shells
            .start_from_tool(
                &json!({
                    "command": "sleep 0.4",
                    "intent": "prerequisite",
                    "dependsOnCapabilities": ["api.http"]
                }),
                "/tmp",
            )
            .expect("start dependent shell");

        let caps = execute_dynamic_tool_call_with_state(
            &json!({
                "tool": "orchestration_list_workers",
                "arguments": {
                    "filter": "capabilities"
                }
            }),
            "/tmp",
            &state,
        );
        assert_eq!(caps["success"], true);
        let caps_text = caps["contentItems"][0]["text"]
            .as_str()
            .expect("capabilities text");
        assert!(caps_text.contains("Service capability index:"));
        assert!(caps_text.contains("@api.http -> <missing provider> [missing]"));

        let deps = execute_dynamic_tool_call_with_state(
            &json!({
                "tool": "orchestration_list_workers",
                "arguments": {
                    "filter": "dependencies"
                }
            }),
            "/tmp",
            &state,
        );
        assert_eq!(deps["success"], true);
        let deps_text = deps["contentItems"][0]["text"]
            .as_str()
            .expect("dependency text");
        assert!(deps_text.contains("Dependencies:"));
        assert!(!deps_text.contains("Main agent state:"));
        assert!(deps_text.contains("shell:bg-1 -> capability:@api.http"));

        let guidance = execute_dynamic_tool_call_with_state(
            &json!({
                "tool": "orchestration_list_workers",
                "arguments": {
                    "filter": "guidance"
                }
            }),
            "/tmp",
            &state,
        );
        assert_eq!(guidance["success"], true);
        let guidance_text = guidance["contentItems"][0]["text"]
            .as_str()
            .expect("guidance text");
        assert!(guidance_text.contains("missing service capability @api.http"));
        let _ = state
            .orchestration
            .background_shells
            .terminate_all_running();
    }

    #[test]
    fn orchestration_list_dependencies_supports_issue_filters() {
        let mut state = AppState::new(true, false);
        state
            .orchestration
            .background_shells
            .start_from_tool(
                &json!({
                    "command": "sleep 0.4",
                    "intent": "prerequisite",
                    "dependsOnCapabilities": ["api.http"]
                }),
                "/tmp",
            )
            .expect("start dependent shell");

        let missing = execute_dynamic_tool_call_with_state(
            &json!({
                "tool": "orchestration_list_dependencies",
                "arguments": {
                    "filter": "missing"
                }
            }),
            "/tmp",
            &state,
        );
        assert_eq!(missing["success"], true);
        let missing_text = missing["contentItems"][0]["text"]
            .as_str()
            .expect("missing dependency text");
        assert!(missing_text.contains("Dependencies:"));
        assert!(missing_text.contains(
            "shell:bg-1 -> capability:@api.http  [dependsOnCapability:missing, blocking]"
        ));

        let sidecars = execute_dynamic_tool_call_with_state(
            &json!({
                "tool": "orchestration_list_dependencies",
                "arguments": {
                    "filter": "sidecars"
                }
            }),
            "/tmp",
            &state,
        );
        assert_eq!(sidecars["success"], true);
        let sidecar_text = sidecars["contentItems"][0]["text"]
            .as_str()
            .expect("sidecar dependency text");
        assert!(sidecar_text.contains("No sidecar dependency edges tracked right now."));
        let _ = state
            .orchestration
            .background_shells
            .terminate_all_running();
    }

    #[test]
    fn orchestration_list_workers_rejects_unknown_filters() {
        let state = AppState::new(true, false);
        let result = execute_dynamic_tool_call_with_state(
            &json!({
                "tool": "orchestration_list_workers",
                "arguments": {
                    "filter": "weird"
                }
            }),
            "/tmp",
            &state,
        );
        assert_eq!(result["success"], false);
        assert!(
            result["contentItems"][0]["text"]
                .as_str()
                .expect("error")
                .contains("orchestration_list_workers `filter`")
        );
    }

    #[test]
    fn orchestration_list_dependencies_rejects_unknown_filters() {
        let state = AppState::new(true, false);
        let result = execute_dynamic_tool_call_with_state(
            &json!({
                "tool": "orchestration_list_dependencies",
                "arguments": {
                    "filter": "weird"
                }
            }),
            "/tmp",
            &state,
        );
        assert_eq!(result["success"], false);
        assert!(
            result["contentItems"][0]["text"]
                .as_str()
                .expect("error")
                .contains("orchestration_list_dependencies `filter`")
        );
    }

    #[test]
    fn workspace_list_dir_returns_sorted_entries() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(workspace.path().join("src")).expect("mkdir");
        std::fs::write(workspace.path().join("a.txt"), "alpha").expect("write");
        std::fs::write(workspace.path().join("src/lib.rs"), "pub fn demo() {}\n").expect("write");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_list_dir",
                "arguments": {"path": ".", "limit": 10}
            }),
            workspace.path().to_str().expect("utf8 path"),
            &BackgroundShellManager::default(),
        );

        assert_eq!(result["success"], true);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("Directory: ."));
        assert!(text.contains("file  5 bytes"));
        assert!(text.contains("a.txt"));
        assert!(text.contains("dir   -"));
        assert!(text.contains("src"));
    }

    #[test]
    fn background_shell_start_preserves_request_origin_metadata() {
        let manager = BackgroundShellManager::default();
        let result = execute_dynamic_tool_call(
            &json!({
                "threadId": "thread-agent-1",
                "callId": "call-55",
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "prerequisite",
                    "label": "repo build",
                    "dependsOnCapabilities": ["api.http"]
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(result["success"], true);
        let snapshots = manager.snapshots();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(
            snapshots[0].origin.source_thread_id.as_deref(),
            Some("thread-agent-1")
        );
        assert_eq!(
            snapshots[0].origin.source_call_id.as_deref(),
            Some("call-55")
        );
        assert_eq!(snapshots[0].intent.as_str(), "prerequisite");
        assert_eq!(snapshots[0].label.as_deref(), Some("repo build"));
        assert_eq!(
            snapshots[0].dependency_capabilities,
            vec!["api.http".to_string()]
        );
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_send_writes_to_alias_target() {
        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": if cfg!(windows) { "more" } else { "cat" },
                    "intent": "service",
                    "label": "echo shell"
                }
            }),
            "/tmp",
            &manager,
        );
        manager.set_job_alias("bg-1", "dev.api").expect("set alias");

        let send_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_send",
                "arguments": {
                    "jobId": "dev.api",
                    "text": "ping from tool"
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(send_result["success"], true);
        let mut rendered = String::new();
        for _ in 0..40 {
            let poll_result = execute_dynamic_tool_call(
                &json!({
                    "tool": "background_shell_poll",
                    "arguments": {
                        "jobId": "dev.api"
                    }
                }),
                "/tmp",
                &manager,
            );
            rendered = poll_result["contentItems"][0]["text"]
                .as_str()
                .expect("poll text")
                .to_string();
            if rendered.contains("ping from tool") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        assert!(rendered.contains("ping from tool"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_attach_returns_service_metadata() {
        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "service",
                    "label": "dev api",
                    "capabilities": ["api.http"],
                    "protocol": "http",
                    "endpoint": "http://127.0.0.1:4000",
                    "attachHint": "Send HTTP requests to /health",
                    "recipes": [
                        {
                            "name": "health",
                            "description": "Check health",
                            "example": "curl http://127.0.0.1:4000/health",
                            "action": {
                                "type": "http",
                                "method": "GET",
                                "path": "/health"
                            }
                        }
                    ]
                }
            }),
            "/tmp",
            &manager,
        );
        manager.set_job_alias("bg-1", "dev.api").expect("set alias");

        let attach_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_attach",
                "arguments": {
                    "jobId": "@api.http"
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(attach_result["success"], true);
        let rendered = attach_result["contentItems"][0]["text"]
            .as_str()
            .expect("attach text");
        assert!(rendered.contains("Service job: bg-1"));
        assert!(rendered.contains("Capabilities: api.http"));
        assert!(rendered.contains("Protocol: http"));
        assert!(rendered.contains("Endpoint: http://127.0.0.1:4000"));
        assert!(rendered.contains("Attach hint: Send HTTP requests to /health"));
        assert!(rendered.contains("health [http GET /health]: Check health"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_inspect_capability_returns_provider_and_consumer_metadata() {
        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "service",
                    "label": "dev api",
                    "capabilities": ["api.http"],
                    "protocol": "http",
                    "endpoint": "http://127.0.0.1:4000",
                    "recipes": [
                        {
                            "name": "health",
                            "description": "Check health"
                        }
                    ]
                }
            }),
            "/tmp",
            &manager,
        );
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "prerequisite",
                    "label": "integration test",
                    "dependsOnCapabilities": ["api.http"]
                }
            }),
            "/tmp",
            &manager,
        );

        let inspect_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_inspect_capability",
                "arguments": {
                    "capability": "@api.http"
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(inspect_result["success"], true);
        let rendered = inspect_result["contentItems"][0]["text"]
            .as_str()
            .expect("inspect text");
        assert!(rendered.contains("Service capability: @api.http"));
        assert!(rendered.contains("bg-1 (dev api)  [untracked]"));
        assert!(rendered.contains("protocol http"));
        assert!(rendered.contains("endpoint http://127.0.0.1:4000"));
        assert!(rendered.contains("recipes  1"));
        assert!(rendered.contains("bg-2 (integration test)  [satisfied]  blocking=yes"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_list_capabilities_can_filter_issue_classes() {
        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "prerequisite",
                    "dependsOnCapabilities": ["api.http"]
                }
            }),
            "/tmp",
            &manager,
        );

        let inspect_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_list_capabilities",
                "arguments": {
                    "status": "missing"
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(inspect_result["success"], true);
        let rendered = inspect_result["contentItems"][0]["text"]
            .as_str()
            .expect("list text");
        assert!(rendered.contains("@api.http -> <missing provider> [missing]"));
        assert!(rendered.contains("used by bg-1 [missing]"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_list_capabilities_accepts_missing_arguments_object() {
        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "service",
                    "capabilities": ["api.http"]
                }
            }),
            "/tmp",
            &manager,
        );

        let inspect_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_list_capabilities"
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(inspect_result["success"], true);
        let rendered = inspect_result["contentItems"][0]["text"]
            .as_str()
            .expect("list text");
        assert!(rendered.contains("@api.http -> bg-1"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_invoke_recipe_runs_structured_service_action() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 4096];
            let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..bytes]);
            assert!(request.starts_with("GET /health HTTP/1.1\r\n"));
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
            )
            .expect("write response");
        });

        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "service",
                    "capabilities": ["api.health"],
                    "protocol": "http",
                    "endpoint": format!("http://{addr}"),
                    "recipes": [
                        {
                            "name": "health",
                            "description": "Check health",
                            "action": {
                                "type": "http",
                                "method": "GET",
                                "path": "/health"
                            }
                        }
                    ]
                }
            }),
            "/tmp",
            &manager,
        );
        manager.set_job_alias("bg-1", "dev.api").expect("set alias");

        let invoke_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_invoke_recipe",
                "arguments": {
                    "jobId": "@api.health",
                    "recipe": "health"
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(invoke_result["success"], true);
        let rendered = invoke_result["contentItems"][0]["text"]
            .as_str()
            .expect("invoke text");
        assert!(rendered.contains("Action: http GET /health"));
        assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_invoke_recipe_supports_http_headers_body_and_expected_status() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 4096];
            let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..bytes]);
            assert!(request.starts_with("POST /seed HTTP/1.1\r\n"));
            assert!(request.contains("Authorization: Bearer demo\r\n"));
            assert!(request.contains("\r\n\r\nseed=true"));
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 202 Accepted\r\nContent-Length: 7\r\nConnection: close\r\n\r\nseeded!",
            )
            .expect("write response");
        });

        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "service",
                    "protocol": "http",
                    "endpoint": format!("http://{addr}"),
                    "recipes": [
                        {
                            "name": "seed",
                            "description": "Seed the service",
                            "action": {
                                "type": "http",
                                "method": "POST",
                                "path": "/seed",
                                "body": "seed=true",
                                "headers": {
                                    "Authorization": "Bearer demo"
                                },
                                "expectedStatus": 202
                            }
                        }
                    ]
                }
            }),
            "/tmp",
            &manager,
        );

        let invoke_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_invoke_recipe",
                "arguments": {
                    "jobId": "bg-1",
                    "recipe": "seed"
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(invoke_result["success"], true);
        let rendered = invoke_result["contentItems"][0]["text"]
            .as_str()
            .expect("invoke text");
        assert!(rendered.contains("Action: http POST /seed headers=1 body=9b expect=202"));
        assert!(rendered.contains("Status code: 202"));
        assert!(rendered.contains("seeded!"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_invoke_recipe_supports_tcp_actions() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 4096];
            let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..bytes]);
            assert_eq!(request, "PING\n");
            std::io::Write::write_all(&mut stream, b"PONG\n").expect("write response");
        });

        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "service",
                    "protocol": "tcp",
                    "endpoint": format!("tcp://{addr}"),
                    "recipes": [
                        {
                            "name": "ping",
                            "description": "Ping the raw socket service",
                            "action": {
                                "type": "tcp",
                                "payload": "PING",
                                "appendNewline": true,
                                "expectSubstring": "PONG",
                                "readTimeoutMs": 500
                            }
                        }
                    ]
                }
            }),
            "/tmp",
            &manager,
        );

        let invoke_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_invoke_recipe",
                "arguments": {
                    "jobId": "bg-1",
                    "recipe": "ping"
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(invoke_result["success"], true);
        let rendered = invoke_result["contentItems"][0]["text"]
            .as_str()
            .expect("invoke text");
        assert!(
            rendered.contains("Action: tcp payload=\"PING\" newline expect=\"PONG\" timeout=500ms")
        );
        assert!(rendered.contains("Address:"));
        assert!(rendered.contains("PONG"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_invoke_recipe_supports_redis_actions() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 4096];
            let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..bytes]);
            assert_eq!(request, "*1\r\n$4\r\nPING\r\n");
            std::io::Write::write_all(&mut stream, b"+PONG\r\n").expect("write response");
        });

        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "service",
                    "protocol": "redis",
                    "endpoint": format!("tcp://{addr}"),
                    "recipes": [
                        {
                            "name": "ping",
                            "description": "Ping the redis service",
                            "action": {
                                "type": "redis",
                                "command": ["PING"],
                                "expectSubstring": "PONG",
                                "readTimeoutMs": 500
                            }
                        }
                    ]
                }
            }),
            "/tmp",
            &manager,
        );

        let invoke_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_invoke_recipe",
                "arguments": {
                    "jobId": "bg-1",
                    "recipe": "ping"
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(invoke_result["success"], true);
        let rendered = invoke_result["contentItems"][0]["text"]
            .as_str()
            .expect("invoke text");
        assert!(rendered.contains("Action: redis PING expect=\"PONG\" timeout=500ms"));
        assert!(rendered.contains("Type: simple-string"));
        assert!(rendered.contains("Value: PONG"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_invoke_recipe_supports_parameter_args() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 4096];
            let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..bytes]);
            assert_eq!(request, "*2\r\n$3\r\nGET\r\n$5\r\nalpha\r\n");
            std::io::Write::write_all(&mut stream, b"$5\r\nvalue\r\n").expect("write response");
        });

        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": "sleep 0.4",
                    "intent": "service",
                    "protocol": "redis",
                    "endpoint": format!("tcp://{addr}"),
                    "recipes": [
                        {
                            "name": "get",
                            "description": "Get one cache entry",
                            "parameters": [
                                {
                                    "name": "key",
                                    "required": true
                                }
                            ],
                            "action": {
                                "type": "redis",
                                "command": ["GET", "{{key}}"],
                                "expectSubstring": "value",
                                "readTimeoutMs": 500
                            }
                        }
                    ]
                }
            }),
            "/tmp",
            &manager,
        );

        let invoke_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_invoke_recipe",
                "arguments": {
                    "jobId": "bg-1",
                    "recipe": "get",
                    "args": {
                        "key": "alpha"
                    }
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(invoke_result["success"], true);
        let rendered = invoke_result["contentItems"][0]["text"]
            .as_str()
            .expect("invoke text");
        assert!(rendered.contains("Action: redis GET alpha"));
        assert!(rendered.contains("Value: value"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_wait_ready_reports_ready_services() {
        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": if cfg!(windows) {
                        "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
                    } else {
                        "sleep 0.15; printf 'READY\\n'; sleep 0.3"
                    },
                    "intent": "service",
                    "readyPattern": "READY"
                }
            }),
            "/tmp",
            &manager,
        );

        let wait_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_wait_ready",
                "arguments": {
                    "jobId": "bg-1",
                    "timeoutMs": 2_000
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(wait_result["success"], true);
        let rendered = wait_result["contentItems"][0]["text"]
            .as_str()
            .expect("wait text");
        assert!(rendered.contains("Ready pattern: READY"));
        assert!(rendered.contains("ready"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_invoke_recipe_waits_for_ready_pattern_before_http_call() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 4096];
            let bytes = std::io::Read::read(&mut stream, &mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..bytes]);
            assert!(request.starts_with("GET /health HTTP/1.1\r\n"));
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
            )
            .expect("write response");
        });

        let manager = BackgroundShellManager::default();
        execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_start",
                "arguments": {
                    "command": if cfg!(windows) {
                        "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
                    } else {
                        "sleep 0.15; printf 'READY\\n'; sleep 0.3"
                    },
                    "intent": "service",
                    "protocol": "http",
                    "endpoint": format!("http://{addr}"),
                    "readyPattern": "READY",
                    "recipes": [
                        {
                            "name": "health",
                            "action": {
                                "type": "http",
                                "method": "GET",
                                "path": "/health"
                            }
                        }
                    ]
                }
            }),
            "/tmp",
            &manager,
        );

        let started = std::time::Instant::now();
        let invoke_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_invoke_recipe",
                "arguments": {
                    "jobId": "bg-1",
                    "recipe": "health",
                    "waitForReadyMs": 2_000
                }
            }),
            "/tmp",
            &manager,
        );

        assert_eq!(invoke_result["success"], true);
        assert!(started.elapsed() >= std::time::Duration::from_millis(100));
        let rendered = invoke_result["contentItems"][0]["text"]
            .as_str()
            .expect("invoke text");
        assert!(rendered.contains("Readiness: waited"));
        assert!(rendered.contains("Status: HTTP/1.1 200 OK"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn workspace_stat_path_reports_type_and_size() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::write(workspace.path().join("hello.txt"), "alpha").expect("write");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_stat_path",
                "arguments": {"path": "hello.txt"}
            }),
            workspace.path().to_str().expect("utf8 path"),
            &BackgroundShellManager::default(),
        );

        assert_eq!(result["success"], true);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("Path: hello.txt"));
        assert!(text.contains("Type: file"));
        assert!(text.contains("Size: 5 bytes"));
    }

    #[test]
    fn workspace_read_file_returns_line_numbered_content() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::write(workspace.path().join("hello.txt"), "alpha\nbeta\n").expect("write");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_read_file",
                "arguments": {"path": "hello.txt", "startLine": 2}
            }),
            workspace.path().to_str().expect("utf8 path"),
            &BackgroundShellManager::default(),
        );

        assert_eq!(result["success"], true);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("File: hello.txt"));
        assert!(text.contains("   2 | beta"));
    }

    #[test]
    fn workspace_search_text_returns_matching_lines() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::write(workspace.path().join("src.txt"), "alpha\nneedle here\n").expect("write");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_search_text",
                "arguments": {"query": "needle"}
            }),
            workspace.path().to_str().expect("utf8 path"),
            &BackgroundShellManager::default(),
        );

        assert_eq!(result["success"], true);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("Text matches for `needle`:"));
        assert!(text.contains("src.txt:2: needle here"));
    }

    #[test]
    fn workspace_find_files_returns_relative_paths() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(workspace.path().join("src")).expect("mkdir");
        std::fs::write(workspace.path().join("src/lib.rs"), "pub fn demo() {}\n").expect("write");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_find_files",
                "arguments": {"query": "lib"}
            }),
            workspace.path().to_str().expect("utf8 path"),
            &BackgroundShellManager::default(),
        );

        assert_eq!(result["success"], true);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("File matches for `lib`:"));
        assert!(text.contains("src/lib.rs"));
    }

    #[test]
    fn workspace_read_file_rejects_escape_outside_workspace() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::NamedTempFile::new().expect("tempfile");

        let result = execute_dynamic_tool_call(
            &json!({
                "tool": "workspace_read_file",
                "arguments": {"path": outside.path()}
            }),
            workspace.path().to_str().expect("utf8 path"),
            &BackgroundShellManager::default(),
        );

        assert_eq!(result["success"], false);
        let text = result["contentItems"][0]["text"]
            .as_str()
            .expect("text output");
        assert!(text.contains("outside the current workspace"));
    }
}
