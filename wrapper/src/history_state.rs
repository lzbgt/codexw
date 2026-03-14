use serde_json::Value;

use crate::history_text::render_user_message_history;
use crate::history_text::sanitize_history_text;
use crate::state::AppState;
use crate::state::ConversationMessage;
use crate::state::get_string;

pub(crate) struct ResumedHistorySnapshot<'a> {
    pub(crate) conversation_items: Vec<&'a Value>,
    pub(crate) conversation_history: Vec<ConversationMessage>,
    pub(crate) latest_user_message: Option<String>,
    pub(crate) latest_agent_message: Option<String>,
}

pub(crate) fn collect_resumed_history_snapshot(
    turns: &[Value],
    render_limit: usize,
    state_limit: usize,
) -> ResumedHistorySnapshot<'_> {
    let mut conversation_items = Vec::with_capacity(render_limit);
    let mut conversation_history = Vec::with_capacity(state_limit);
    let mut latest_user_message = None;
    let mut latest_agent_message = None;

    'outer: for turn in turns.iter().rev() {
        if let Some(turn_items) = turn.get("items").and_then(Value::as_array) {
            for item in turn_items.iter().rev() {
                if let Some(message) = conversation_history_entry(item) {
                    if conversation_items.len() < render_limit {
                        conversation_items.push(item);
                    }
                    if conversation_history.len() < state_limit {
                        conversation_history.push(message.clone());
                    }
                    match message.role.as_str() {
                        "user" if latest_user_message.is_none() => {
                            latest_user_message = Some(message.text.clone());
                        }
                        "assistant" if latest_agent_message.is_none() => {
                            latest_agent_message = Some(message.text.clone());
                        }
                        _ => {}
                    }
                }
                if conversation_items.len() >= render_limit
                    && conversation_history.len() >= state_limit
                    && latest_user_message.is_some()
                    && latest_agent_message.is_some()
                {
                    break 'outer;
                }
            }
        }
    }

    conversation_items.reverse();
    conversation_history.reverse();
    ResumedHistorySnapshot {
        conversation_items,
        conversation_history,
        latest_user_message,
        latest_agent_message,
    }
}

#[cfg(test)]
pub(crate) fn latest_conversation_history_items(turns: &[Value], limit: usize) -> Vec<&Value> {
    collect_resumed_history_snapshot(turns, limit, 0).conversation_items
}

fn conversation_history_entry(item: &Value) -> Option<ConversationMessage> {
    match get_string(item, &["type"]).unwrap_or("") {
        "userMessage" => item
            .get("content")
            .and_then(Value::as_array)
            .map(|content| render_user_message_history(content))
            .filter(|text| !text.trim().is_empty())
            .map(|text| ConversationMessage {
                role: "user".to_string(),
                text,
            }),
        "agentMessage" => {
            let text = sanitize_history_text(get_string(item, &["text"]).unwrap_or(""));
            if text.trim().is_empty() {
                None
            } else {
                Some(ConversationMessage {
                    role: "assistant".to_string(),
                    text,
                })
            }
        }
        _ => None,
    }
}

pub(crate) fn seed_resumed_state_from_snapshot(
    snapshot: &ResumedHistorySnapshot<'_>,
    state: &mut AppState,
) {
    state.replace_conversation_history(snapshot.conversation_history.clone());
    if let Some(message) = snapshot.latest_user_message.as_ref() {
        state.objective = Some(message.clone());
    }
    if let Some(message) = snapshot.latest_agent_message.as_ref() {
        state.last_agent_message = Some(message.clone());
    }
}

#[cfg(test)]
pub(crate) fn seed_resumed_state_from_turns(turns: &[Value], state: &mut AppState) {
    let snapshot = collect_resumed_history_snapshot(turns, 0, 50);
    seed_resumed_state_from_snapshot(&snapshot, state);
}
