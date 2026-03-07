use anyhow::Result;
use serde_json::Value;

use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;

pub(crate) fn render_resumed_history(
    result: &Value,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    let turns = result
        .get("thread")
        .and_then(|thread| thread.get("turns"))
        .and_then(Value::as_array);
    let Some(turns) = turns else {
        return Ok(());
    };
    if turns.is_empty() {
        return Ok(());
    }

    seed_resumed_state_from_turns(turns, state);
    let conversation_items = latest_conversation_history_items(turns, 10);
    if conversation_items.is_empty() {
        return Ok(());
    }

    output.line_stderr("[history] showing latest 10 conversation messages from resumed thread")?;
    for item in conversation_items {
        render_history_item(item, state, output)?;
    }
    Ok(())
}

pub(crate) fn latest_conversation_history_items<'a>(
    turns: &'a [Value],
    limit: usize,
) -> Vec<&'a Value> {
    let mut items = Vec::with_capacity(limit);
    for turn in turns.iter().rev() {
        if let Some(turn_items) = turn.get("items").and_then(Value::as_array) {
            for item in turn_items.iter().rev() {
                if is_conversation_history_item(item) {
                    items.push(item);
                    if items.len() == limit {
                        items.reverse();
                        return items;
                    }
                }
            }
        }
    }
    items.reverse();
    items
}

fn is_conversation_history_item(item: &Value) -> bool {
    match get_string(item, &["type"]).unwrap_or("") {
        "userMessage" => item
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|content| !content.is_empty()),
        "agentMessage" => item
            .get("text")
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty()),
        _ => false,
    }
}

pub(crate) fn seed_resumed_state_from_turns(turns: &[Value], state: &mut AppState) {
    let mut latest_user_message = None;
    let mut latest_agent_message = None;

    'outer: for turn in turns.iter().rev() {
        if let Some(items) = turn.get("items").and_then(Value::as_array) {
            for item in items.iter().rev() {
                match get_string(item, &["type"]).unwrap_or("") {
                    "userMessage" if latest_user_message.is_none() => {
                        if let Some(content) = item.get("content").and_then(Value::as_array) {
                            let rendered = render_user_message_history(content);
                            if !rendered.trim().is_empty() {
                                latest_user_message = Some(rendered);
                            }
                        }
                    }
                    "agentMessage" if latest_agent_message.is_none() => {
                        let text = sanitize_history_text(get_string(item, &["text"]).unwrap_or(""));
                        if !text.trim().is_empty() {
                            latest_agent_message = Some(text);
                        }
                    }
                    _ => {}
                }

                if latest_user_message.is_some() && latest_agent_message.is_some() {
                    break 'outer;
                }
            }
        }
    }

    if let Some(message) = latest_user_message {
        state.objective = Some(message);
    }
    if let Some(message) = latest_agent_message {
        state.last_agent_message = Some(message);
    }
}

fn render_history_item(item: &Value, state: &mut AppState, output: &mut Output) -> Result<()> {
    match get_string(item, &["type"]).unwrap_or("") {
        "userMessage" => {
            let content = item
                .get("content")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let rendered = render_user_message_history(&content);
            if !rendered.trim().is_empty() {
                output.block_stdout("User", &rendered)?;
            }
        }
        "agentMessage" => {
            let text = sanitize_history_text(get_string(item, &["text"]).unwrap_or(""));
            if !text.trim().is_empty() {
                state.last_agent_message = Some(text.clone());
                output.block_stdout("Assistant", &text)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn render_user_message_history(content: &[Value]) -> String {
    let mut parts = Vec::new();
    for item in content {
        match get_string(item, &["type"]).unwrap_or("") {
            "text" => {
                if let Some(text) = get_string(item, &["text"]) {
                    parts.push(text.to_string());
                }
            }
            "image" => {
                if let Some(url) = get_string(item, &["imageUrl"]) {
                    parts.push(format!("[image] {url}"));
                }
            }
            "localImage" => {
                if let Some(path) = get_string(item, &["path"]) {
                    parts.push(format!("[local-image] {path}"));
                }
            }
            "mention" => {
                let label = get_string(item, &["label"]).unwrap_or("$mention");
                let uri = get_string(item, &["uri"]).unwrap_or("");
                if uri.is_empty() {
                    parts.push(label.to_string());
                } else {
                    parts.push(format!("{label} ({uri})"));
                }
            }
            "skill" => {
                if let Some(path) = get_string(item, &["path"]) {
                    parts.push(format!("[skill] {path}"));
                }
            }
            _ => {}
        }
    }
    sanitize_history_text(&parts.join("\n"))
}

fn sanitize_history_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let min_indent = lines
        .iter()
        .filter_map(|line| {
            if line.trim().is_empty() {
                None
            } else {
                Some(
                    line.chars()
                        .take_while(|ch| *ch == ' ' || *ch == '\t')
                        .count(),
                )
            }
        })
        .min()
        .unwrap_or(0);
    let cleaned = lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                line.chars().skip(min_indent).collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    cleaned
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
