use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;

use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;

pub(crate) fn handle_review_start(
    state: &mut AppState,
    output: &mut Output,
    target_description: &str,
) -> Result<()> {
    state.turn_running = true;
    let now = Instant::now();
    state.activity_started_at = Some(now);
    state.last_server_event_at = Some(now);
    state.turn_idle_notice_emitted = false;
    state.reset_turn_stream_state();
    output.line_stderr(format!(
        "[review] started {}",
        summarize_text(target_description)
    ))?;
    Ok(())
}

pub(crate) fn handle_turn_start(
    result: &Value,
    state: &mut AppState,
    output: &mut Output,
    auto_generated: bool,
) -> Result<()> {
    let turn_id = get_string(result, &["turn", "id"])
        .context("turn/start missing turn.id")?
        .to_string();
    state.active_turn_id = Some(turn_id);
    state.turn_running = true;
    let now = Instant::now();
    state.activity_started_at = Some(now);
    state.last_server_event_at = Some(now);
    state.turn_idle_notice_emitted = false;
    state.reset_turn_stream_state();
    if auto_generated {
        output.line_stderr("[auto] starting follow-up turn")?;
    }
    Ok(())
}

pub(crate) fn handle_turn_steer(
    result: &Value,
    state: &mut AppState,
    output: &mut Output,
    display_text: &str,
) -> Result<()> {
    let turn_id = get_string(result, &["turnId"])
        .context("turn/steer missing turnId")?
        .to_string();
    state.active_turn_id = Some(turn_id);
    output.line_stderr(format!("[steer] {}", summarize_text(display_text)))?;
    Ok(())
}

pub(crate) fn handle_turn_interrupt(output: &mut Output) -> Result<()> {
    output.line_stderr("[interrupt] requested")?;
    Ok(())
}
