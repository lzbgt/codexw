use serde_json::Value;

use crate::state::get_string;
use crate::state::summarize_text;
use crate::status_views::summarize_value;

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
