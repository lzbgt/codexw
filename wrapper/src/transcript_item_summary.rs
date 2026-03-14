use serde_json::Value;

use crate::client_dynamic_tools::is_legacy_workspace_tool;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::status_value::summarize_value;
use crate::transcript_completion_render::abbreviate_long_result_text;

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

pub(crate) fn humanize_item_type(item_type: &str) -> String {
    match item_type {
        "todoList" => "Todo list".to_string(),
        "externalToolCall" => "Tool call".to_string(),
        "commandExecution" => "Command".to_string(),
        "localShellCall" => "Local shell".to_string(),
        "fileChange" => "File change".to_string(),
        "mcpToolCall" => "MCP tool".to_string(),
        "dynamicToolCall" => "Dynamic tool".to_string(),
        "collabAgentToolCall" => "Agent collaboration".to_string(),
        "webSearch" => "Web search".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn summarize_tool_item(item_type: &str, item: &Value, verbose: bool) -> String {
    let summary = match item_type {
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
        "mcpToolCall" | "dynamicToolCall" | "collabAgentToolCall" | "webSearch" => {
            summarize_tool_result_block(item).unwrap_or_else(|| summarize_value(item))
        }
        "thinking" | "reasoning" => "reasoning".to_string(),
        "imageGeneration" => get_string(item, &["prompt"])
            .map(|prompt| format!("image prompt {}", summarize_text(prompt)))
            .unwrap_or_else(|| "image generation".to_string()),
        _ => summarize_value(item),
    };
    abbreviate_long_result_text(&summary, verbose)
}

fn summarize_tool_result_block(item: &Value) -> Option<String> {
    let title = get_string(item, &["title"])
        .or_else(|| get_string(item, &["toolName"]))
        .or_else(|| get_string(item, &["tool"]))
        .or_else(|| get_string(item, &["command"]))
        .map(render_tool_summary_title);
    let body = extract_tool_result_text(item);

    match (title, body) {
        (Some(title), Some(body)) if !body.trim().is_empty() => Some(format!("{title}\n\n{body}")),
        (Some(title), _) => Some(title.to_string()),
        (None, Some(body)) if !body.trim().is_empty() => Some(body),
        _ => None,
    }
}

fn render_tool_summary_title(title: &str) -> String {
    if is_legacy_workspace_tool(title) {
        format!("{title} (legacy workspace compatibility)")
    } else {
        title.to_string()
    }
}

fn extract_tool_result_text(item: &Value) -> Option<String> {
    if let Some(text) = extract_text_array(item.get("contentItems")) {
        return Some(text);
    }
    if let Some(text) = extract_text_array(item.get("content")) {
        return Some(text);
    }
    if let Some(text) =
        extract_text_array(item.get("result").and_then(|result| result.get("content")))
    {
        return Some(text);
    }
    if let Some(text) = get_string(item, &["error"]) {
        return Some(text.to_string());
    }
    if let Some(text) = get_string(item, &["result", "error", "message"]) {
        return Some(text.to_string());
    }
    if let Some(text) = get_string(item, &["result", "message"]) {
        return Some(text.to_string());
    }
    None
}

fn extract_text_array(value: Option<&Value>) -> Option<String> {
    let texts = value
        .and_then(Value::as_array)?
        .iter()
        .filter_map(extract_text_from_value)
        .collect::<Vec<_>>();
    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n\n"))
    }
}

fn extract_text_from_value(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    get_string(value, &["text"])
        .or_else(|| get_string(value, &["content"]))
        .or_else(|| get_string(value, &["message"]))
        .map(ToString::to_string)
}
