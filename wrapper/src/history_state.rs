use serde_json::Value;

use crate::history_text::render_user_message_history;
use crate::history_text::sanitize_history_text;
use crate::state::AppState;
use crate::state::get_string;

pub(crate) fn latest_conversation_history_items(turns: &[Value], limit: usize) -> Vec<&Value> {
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
