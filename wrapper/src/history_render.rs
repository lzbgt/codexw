use anyhow::Result;
use serde_json::Value;

use crate::history_text::render_user_message_history;
use crate::history_text::sanitize_history_text;
use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;

use crate::history_state::collect_resumed_history_snapshot;
use crate::history_state::seed_resumed_state_from_snapshot;

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

    let snapshot = collect_resumed_history_snapshot(turns, 10, 50);
    seed_resumed_state_from_snapshot(&snapshot, state);
    if snapshot.conversation_items.is_empty() {
        return Ok(());
    }

    output.line_stderr("[history] showing latest 10 conversation messages from resumed thread")?;
    for item in snapshot.conversation_items {
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
