use serde_json::Value;

use crate::state::get_string;
use crate::state::summarize_text;
use crate::status_value::summarize_value;

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
