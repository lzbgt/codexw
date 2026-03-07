use chrono::DateTime;
use chrono::Local;
use chrono::Utc;
use serde_json::Value;
use serde_json::json;

use super::Cli;
use super::approval_policy;
use super::thread_sandbox_mode;
use super::turn_sandbox_policy;
use crate::input::AppCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::session::extract_models;
use crate::state::get_string;
use crate::state::summarize_text;

pub(crate) fn render_command_completion(
    command: &str,
    status: &str,
    exit_code: &str,
    output: Option<&str>,
) -> String {
    let mut rendered = format!("{command}\n[status] {status}  [exit] {exit_code}");
    if let Some(output) = output {
        let trimmed = output.trim_end();
        if !trimmed.is_empty() {
            rendered.push_str("\n\n");
            rendered.push_str(trimmed);
        }
    }
    rendered
}

pub(crate) fn render_local_command_completion(
    command: &str,
    exit_code: &str,
    stdout: &str,
    stderr: &str,
) -> String {
    let mut rendered = format!("{command}\n[exit] {exit_code}");
    if !stdout.trim().is_empty() {
        rendered.push_str("\n\n[stdout]\n");
        rendered.push_str(stdout.trim_end());
    }
    if !stderr.trim().is_empty() {
        rendered.push_str("\n\n[stderr]\n");
        rendered.push_str(stderr.trim_end());
    }
    rendered
}

pub(crate) fn render_file_change_completion(
    item: &Value,
    status: &str,
    output: Option<&str>,
) -> String {
    let mut rendered = format!("[status] {status}\n{}", summarize_file_change_paths(item));
    let structured = render_file_changes(item);
    if !structured.is_empty() {
        rendered.push_str("\n\n");
        rendered.push_str(&structured);
    } else if let Some(output) = output {
        let trimmed = output.trim_end();
        if !trimmed.is_empty() {
            rendered.push_str("\n\n");
            rendered.push_str(trimmed);
        }
    }
    rendered
}

pub(crate) fn render_experimental_features_list(result: &Value) -> String {
    let mut lines = Vec::new();
    let features = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for feature in features {
        let name = feature.get("name").and_then(Value::as_str).unwrap_or("?");
        let stage = feature
            .get("stage")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let enabled = feature
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let default_enabled = feature
            .get("defaultEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let display_name = feature
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or(name);
        let description = feature
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let status = if enabled {
            "enabled"
        } else if default_enabled {
            "default-on"
        } else {
            "disabled"
        };

        lines.push(format!("{display_name}  [{stage}] [{status}]"));
        lines.push(format!("  key: {name}"));
        if !description.is_empty() {
            lines.push(format!("  {description}"));
        }
        if let Some(announcement) = feature.get("announcement").and_then(Value::as_str)
            && !announcement.trim().is_empty()
        {
            lines.push(format!("  note: {}", summarize_text(announcement)));
        }
        lines.push(String::new());
    }

    if lines.is_empty() {
        lines.push("No experimental features were returned by app-server.".to_string());
    } else {
        lines.pop();
    }

    if result.get("nextCursor").and_then(Value::as_str).is_some() {
        lines.push(String::new());
        lines.push("More feature entries are available from app-server.".to_string());
    }

    lines.join("\n")
}

pub(crate) fn render_pending_attachments(
    local_images: &[String],
    remote_images: &[String],
) -> String {
    let mut lines = Vec::new();
    for path in local_images {
        lines.push(format!("local-image  {path}"));
    }
    for url in remote_images {
        lines.push(format!("remote-image {url}"));
    }
    lines.join("\n")
}

pub(crate) fn render_permissions_snapshot(cli: &Cli) -> String {
    [
        format!("approval policy  {}", approval_policy(cli)),
        format!("thread sandbox   {}", thread_sandbox_mode(cli)),
        format!(
            "turn sandbox     {}",
            summarize_sandbox_policy(&turn_sandbox_policy(cli))
        ),
        "network access    enabled".to_string(),
        "tool use          automatic".to_string(),
        "shell exec        automatic".to_string(),
        "host access       full".to_string(),
    ]
    .join("\n")
}

pub(crate) fn render_config_snapshot(result: &Value) -> String {
    if result.is_null() {
        return "config unavailable".to_string();
    }
    serde_json::to_string_pretty(result).unwrap_or_else(|_| summarize_value(result))
}

pub(crate) fn summarize_sandbox_policy(policy: &Value) -> String {
    match get_string(policy, &["type"]).unwrap_or("unknown") {
        "dangerFullAccess" => "dangerFullAccess".to_string(),
        other => summarize_value(&json!({
            "type": other,
            "policy": policy,
        })),
    }
}

pub(crate) fn render_account_summary(account: Option<&Value>) -> Option<String> {
    let account = account?;
    if account.is_null() {
        return Some("not signed in".to_string());
    }
    let account_type = get_string(account, &["type"])
        .or_else(|| get_string(account, &["authMode"]))
        .unwrap_or("unknown");
    let mut parts = vec![account_type.to_string()];
    if let Some(email) = get_string(account, &["email"]) {
        parts.push(email.to_string());
    }
    if let Some(plan_type) = get_string(account, &["planType"]) {
        parts.push(format!("plan={plan_type}"));
    }
    Some(parts.join(" "))
}

pub(crate) fn render_rate_limit_lines(rate_limits: Option<&Value>) -> Vec<String> {
    let Some(rate_limits) = rate_limits else {
        return vec!["rate limits     unavailable".to_string()];
    };

    let mut lines = Vec::new();
    let mut first_row = true;
    for (label, window_key) in [("primary", "primary"), ("secondary", "secondary")] {
        let Some(window) = rate_limits.get(window_key) else {
            continue;
        };
        if window.is_null() {
            continue;
        }
        let used_percent = window
            .get("usedPercent")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let percent_left = (100.0 - used_percent).clamp(0.0, 100.0);
        let window_minutes = window.get("windowDurationMins").and_then(Value::as_i64);
        let duration_label = window_minutes
            .map(get_limits_duration)
            .unwrap_or_else(|| label.to_string());
        let reset_label = window
            .get("resetsAt")
            .and_then(Value::as_i64)
            .and_then(format_reset_timestamp_local);
        let mut line = format!(
            "{}{} limit {}",
            if first_row {
                "rate limits     "
            } else {
                "                "
            },
            duration_label,
            format_status_limit_summary(percent_left),
        );
        if let Some(reset_label) = reset_label {
            line.push_str(&format!(" (resets {reset_label})"));
        }
        lines.push(line);
        first_row = false;
    }

    if let Some(credits) = rate_limits.get("credits")
        && let Some(credit_line) = render_credit_line(credits, first_row)
    {
        lines.push(credit_line);
    }

    if lines.is_empty() {
        vec!["rate limits     none reported".to_string()]
    } else {
        lines
    }
}

fn render_credit_line(credits: &Value, first_row: bool) -> Option<String> {
    if !credits
        .get("hasCredits")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let prefix = if first_row {
        "rate limits     "
    } else {
        "                "
    };
    if credits
        .get("unlimited")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Some(format!("{prefix}credits unlimited"));
    }
    let balance = credits.get("balance").and_then(Value::as_str)?.trim();
    if balance.is_empty() {
        return None;
    }
    Some(format!("{prefix}credits {balance}"))
}

pub(crate) fn render_fuzzy_file_search_results(query: &str, files: &[Value]) -> String {
    if files.is_empty() {
        return format!("No files matched \"{query}\".");
    }
    let mut lines = vec![format!("Query: {query}")];
    for (index, file) in files.iter().take(20).enumerate() {
        let path = get_string(file, &["path"]).unwrap_or("?");
        let score = file
            .get("score")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        lines.push(format!("{:>2}. {}  [score {}]", index + 1, path, score));
    }
    if files.len() > 20 {
        lines.push(format!("...and {} more", files.len() - 20));
    }
    lines.push("Use /mention <n> to insert a match into the prompt.".to_string());
    lines.join("\n")
}

pub(crate) fn render_apps_list(apps: &[AppCatalogEntry]) -> String {
    if apps.is_empty() {
        return "No apps are currently available.".to_string();
    }
    apps.iter()
        .map(|app| {
            format!(
                "{}  ${}  [{}]",
                app.name,
                app.slug,
                if app.enabled { "enabled" } else { "disabled" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_skills_list(skills: &[SkillCatalogEntry]) -> String {
    if skills.is_empty() {
        return "No skills found for the current workspace.".to_string();
    }
    skills
        .iter()
        .map(|skill| {
            format!(
                "{}  {}  [{}]",
                skill.name,
                skill.path,
                if skill.enabled { "enabled" } else { "disabled" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_models_list(result: &Value) -> String {
    let models = extract_models(result);
    if models.is_empty() {
        return "No models returned by app-server.".to_string();
    }
    models
        .iter()
        .take(30)
        .map(|model| {
            let default_marker = if model.is_default { " [default]" } else { "" };
            let personality_marker = if model.supports_personality {
                " [supports personality]"
            } else {
                " [personality unsupported]"
            };
            format!(
                "{} ({}){}{}",
                model.display_name, model.id, default_marker, personality_marker
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_mcp_server_list(result: &Value) -> String {
    let entries = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if entries.is_empty() {
        return "No MCP servers returned by app-server.".to_string();
    }
    entries
        .iter()
        .map(|entry| {
            let name = get_string(entry, &["name"]).unwrap_or("?");
            let auth = get_string(entry, &["authStatus"])
                .or_else(|| get_string(entry, &["auth", "status"]))
                .unwrap_or("unknown");
            let tools = entry
                .get("tools")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0);
            let resources = entry
                .get("resources")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0);
            format!("{name}  [auth {auth}]  [tools {tools}]  [resources {resources}]")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_thread_list(result: &Value, search_term: Option<&str>) -> String {
    let threads = result
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if threads.is_empty() {
        return match search_term {
            Some(search_term) => format!("No threads matched \"{search_term}\"."),
            None => "No threads found for the current workspace.".to_string(),
        };
    }
    let mut lines = Vec::new();
    if let Some(search_term) = search_term {
        lines.push(format!("Search: {search_term}"));
    }
    lines.extend(threads.iter().enumerate().map(|(index, thread)| {
        let id = get_string(thread, &["id"]).unwrap_or("?");
        let preview = get_string(thread, &["preview"]).unwrap_or("-");
        let status = get_string(thread, &["status", "type"]).unwrap_or("unknown");
        let updated_at = thread
            .get("updatedAt")
            .and_then(Value::as_i64)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        format!(
            "{:>2}. {id}  [{status}]  [updated {updated_at}]  {}",
            index + 1,
            summarize_text(preview)
        )
    }));
    lines.push("Use /resume <n> to resume one of these threads.".to_string());
    lines.join("\n")
}

pub(crate) fn extract_thread_ids(result: &Value) -> Vec<String> {
    result
        .get("data")
        .and_then(Value::as_array)
        .map(|threads| {
            threads
                .iter()
                .filter_map(|thread| get_string(thread, &["id"]).map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn extract_file_search_paths(files: &[Value]) -> Vec<String> {
    files
        .iter()
        .filter_map(|file| get_string(file, &["path"]).map(ToOwned::to_owned))
        .collect()
}

pub(crate) fn render_token_usage_summary(token_usage: Option<&Value>) -> Option<String> {
    let token_usage = token_usage?;
    let last_total = token_usage
        .get("last")
        .and_then(|value| value.get("totalTokens"))
        .and_then(Value::as_u64);
    let cumulative_total = token_usage
        .get("total")
        .and_then(|value| value.get("totalTokens"))
        .and_then(Value::as_u64);
    match (last_total, cumulative_total) {
        (Some(last_total), Some(cumulative_total)) => {
            Some(format!("last={} total={}", last_total, cumulative_total))
        }
        (Some(last_total), None) => Some(format!("last={last_total}")),
        (None, Some(cumulative_total)) => Some(format!("total={cumulative_total}")),
        (None, None) => None,
    }
}

fn get_limits_duration(window_minutes: i64) -> String {
    const MINUTES_PER_HOUR: i64 = 60;
    const MINUTES_PER_DAY: i64 = 24 * MINUTES_PER_HOUR;
    const MINUTES_PER_WEEK: i64 = 7 * MINUTES_PER_DAY;
    const MINUTES_PER_MONTH: i64 = 30 * MINUTES_PER_DAY;
    const ROUNDING_BIAS_MINUTES: i64 = 3;

    let window_minutes = window_minutes.max(0);
    if window_minutes <= MINUTES_PER_DAY.saturating_add(ROUNDING_BIAS_MINUTES) {
        let adjusted = window_minutes.saturating_add(ROUNDING_BIAS_MINUTES);
        let hours = std::cmp::max(1, adjusted / MINUTES_PER_HOUR);
        format!("{hours}h")
    } else if window_minutes <= MINUTES_PER_WEEK.saturating_add(ROUNDING_BIAS_MINUTES) {
        "weekly".to_string()
    } else if window_minutes <= MINUTES_PER_MONTH.saturating_add(ROUNDING_BIAS_MINUTES) {
        "monthly".to_string()
    } else {
        "annual".to_string()
    }
}

fn format_status_limit_summary(percent_remaining: f64) -> String {
    format!("{percent_remaining:.0}% left")
}

fn format_reset_timestamp_local(unix_seconds: i64) -> Option<String> {
    let dt_utc = DateTime::<Utc>::from_timestamp(unix_seconds, 0)?;
    let dt_local = dt_utc.with_timezone(&Local);
    let now = Local::now();
    let time = dt_local.format("%H:%M").to_string();
    if dt_local.date_naive() == now.date_naive() {
        Some(time)
    } else {
        Some(format!("{time} on {}", dt_local.format("%-d %b")))
    }
}

pub(crate) fn render_file_changes(item: &Value) -> String {
    let Some(changes) = item.get("changes").and_then(Value::as_array) else {
        return String::new();
    };
    let mut rendered = String::new();
    for (idx, change) in changes.iter().enumerate() {
        if idx > 0 {
            rendered.push_str("\n\n");
        }
        let path = get_string(change, &["path"]).unwrap_or("?");
        let kind = get_string(change, &["kind"]).unwrap_or("?");
        rendered.push_str(&format!("{kind} {path}"));
        if let Some(diff) = get_string(change, &["diff"])
            && !diff.is_empty()
        {
            rendered.push_str("\n\n");
            rendered.push_str(diff);
        }
    }
    rendered
}

pub(crate) fn summarize_file_change_paths(item: &Value) -> String {
    let Some(changes) = item.get("changes").and_then(Value::as_array) else {
        return "updating files".to_string();
    };
    let paths = changes
        .iter()
        .filter_map(|change| get_string(change, &["path"]))
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return "updating files".to_string();
    }
    let preview = paths.iter().take(3).copied().collect::<Vec<_>>().join(", ");
    if paths.len() <= 3 {
        format!("updating {}", preview)
    } else {
        format!("updating {} and {} more", preview, paths.len() - 3)
    }
}

pub(crate) fn format_plan(params: &Value) -> String {
    params
        .get("plan")
        .and_then(Value::as_array)
        .map(|plan| {
            plan.iter()
                .map(|step| {
                    let step_text = get_string(step, &["step"]).unwrap_or("-");
                    let status = get_string(step, &["status"]).unwrap_or("pending");
                    format!("- [{status}] {step_text}")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

pub(crate) fn render_reasoning_item(item: &Value) -> String {
    let summary = item
        .get("summary")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !summary.is_empty() {
        return summary.join("\n\n");
    }

    item.get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default()
}

pub(crate) fn build_tool_user_input_response(params: &Value) -> Value {
    let mut answers = serde_json::Map::new();
    if let Some(questions) = params.get("questions").and_then(Value::as_array) {
        for question in questions {
            let Some(id) = get_string(question, &["id"]) else {
                continue;
            };
            let selected = question
                .get("options")
                .and_then(Value::as_array)
                .and_then(|options| options.first())
                .and_then(|first| get_string(first, &["label"]))
                .map(|label| vec![label.to_string()])
                .unwrap_or_else(|| vec!["".to_string()]);
            answers.insert(id.to_string(), json!({ "answers": selected }));
        }
    }
    Value::Object(
        [("answers".to_string(), Value::Object(answers))]
            .into_iter()
            .collect(),
    )
}

pub(crate) fn summarize_command_approval_request(params: &Value, decision: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(reason) = get_string(params, &["reason"]) {
        parts.push(format!("reason={reason}"));
    }
    if let Some(command) = get_string(params, &["command"]) {
        parts.push(format!("command={command}"));
    }
    if let Some(cwd) = get_string(params, &["cwd"]) {
        parts.push(format!("cwd={cwd}"));
    }
    if let Some(host) = get_string(params, &["networkApprovalContext", "host"]) {
        parts.push(format!("network_host={host}"));
    }
    parts.push(format!("decision={}", summarize_value(decision)));
    parts.join(" ")
}

pub(crate) fn summarize_generic_approval_request(params: &Value, method: &str) -> String {
    let mut parts = vec![method.to_string()];
    if let Some(reason) = get_string(params, &["reason"]) {
        parts.push(format!("reason={reason}"));
    }
    if let Some(root) = get_string(params, &["grantRoot"]) {
        parts.push(format!("grant_root={root}"));
    }
    if let Some(cwd) = get_string(params, &["cwd"]) {
        parts.push(format!("cwd={cwd}"));
    }
    parts.join(" ")
}

pub(crate) fn summarize_tool_request(params: &Value) -> String {
    if let Some(message) = get_string(params, &["message"]) {
        return message.to_string();
    }
    if let Some(questions) = params.get("questions").and_then(Value::as_array) {
        let rendered = questions
            .iter()
            .filter_map(|question| get_string(question, &["question"]))
            .collect::<Vec<_>>();
        if !rendered.is_empty() {
            return rendered.join(" | ");
        }
    }
    summarize_value(params)
}

pub(crate) fn summarize_thread_status_for_display(params: &Value) -> Option<String> {
    let status_type = get_string(params, &["status", "type"]).unwrap_or("unknown");
    let flags = params
        .get("status")
        .and_then(|v| v.get("activeFlags"))
        .and_then(Value::as_array)
        .map(|flags| flags.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();

    if status_type == "active" && flags.is_empty() {
        return None;
    }

    if flags.iter().any(|flag| *flag == "waitingOnApproval") {
        return Some("waiting on approval".to_string());
    }

    if flags.is_empty() {
        if status_type == "idle" {
            Some("ready".to_string())
        } else {
            Some(status_type.to_string())
        }
    } else {
        Some(flags.join(", "))
    }
}

pub(crate) fn summarize_model_reroute(params: &Value) -> String {
    let from_model = get_string(params, &["fromModel"]).unwrap_or("?");
    let to_model = get_string(params, &["toModel"]).unwrap_or("?");
    let reason = get_string(params, &["reason"]).unwrap_or("unspecified");
    format!("{from_model} -> {to_model} reason={reason}")
}

pub(crate) fn summarize_terminal_interaction(params: &Value) -> Option<String> {
    let process_id = get_string(params, &["processId"]).unwrap_or("?");
    let stdin = get_string(params, &["stdin"])?.trim();
    if stdin.is_empty() {
        return None;
    }
    Some(format!(
        "process={process_id} stdin={}",
        summarize_text(stdin)
    ))
}

pub(crate) fn summarize_server_request_resolved(params: &Value) -> String {
    let thread_id = get_string(params, &["threadId"]).unwrap_or("?");
    let request_id = params
        .get("requestId")
        .map(summarize_value)
        .unwrap_or_else(|| "?".to_string());
    format!("thread={thread_id} request={request_id}")
}

pub(crate) fn humanize_item_type(item_type: &str) -> String {
    match item_type {
        "todoList" => "Todo list".to_string(),
        "externalToolCall" => "Tool call".to_string(),
        "commandExecution" => "Command".to_string(),
        "localShellCall" => "Local shell".to_string(),
        "fileChange" => "File change".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn summarize_tool_item(item_type: &str, item: &Value) -> String {
    match item_type {
        "todoList" => item
            .get("items")
            .and_then(Value::as_array)
            .map(|items| format!("{} todo items", items.len()))
            .unwrap_or_else(|| "todo list".to_string()),
        "externalToolCall" | "localShellCall" => get_string(item, &["title"])
            .or_else(|| get_string(item, &["toolName"]))
            .or_else(|| get_string(item, &["command"]))
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "tool call".to_string()),
        "commandExecution" => get_string(item, &["command"])
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "command".to_string()),
        "fileChange" => summarize_file_change_paths(item),
        "thinking" | "reasoning" => "reasoning".to_string(),
        "imageGeneration" => get_string(item, &["prompt"])
            .map(|prompt| format!("image prompt {}", summarize_text(prompt)))
            .unwrap_or_else(|| "image generation".to_string()),
        _ => summarize_value(item),
    }
}

pub(crate) fn summarize_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => string.to_string(),
        Value::Array(array) => {
            if array.is_empty() {
                "[]".to_string()
            } else {
                format!("[{} items]", array.len())
            }
        }
        Value::Object(object) => object
            .iter()
            .take(6)
            .map(|(key, value)| format!("{key}={}", summarize_value(value)))
            .collect::<Vec<_>>()
            .join(" "),
    }
}
