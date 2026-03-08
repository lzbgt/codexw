use anyhow::Result;
use serde_json::Value;

use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;

use crate::history_state::latest_conversation_history_items;
use crate::history_state::sanitize_history_text;
use crate::history_state::seed_resumed_state_from_turns;

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
