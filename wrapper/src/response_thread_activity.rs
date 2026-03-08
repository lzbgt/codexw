use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;

use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;
use crate::transcript_render::render_local_command_completion;

pub(crate) fn handle_realtime_start(
    state: &mut AppState,
    output: &mut Output,
    prompt: &str,
) -> Result<()> {
    state.realtime_prompt = Some(prompt.to_string());
    output.line_stderr("[realtime] start requested")?;
    Ok(())
}

pub(crate) fn handle_realtime_append(output: &mut Output, text: &str) -> Result<()> {
    output.line_stderr(format!("[realtime] sent {}", summarize_text(text)))?;
    Ok(())
}

pub(crate) fn handle_realtime_stop(output: &mut Output) -> Result<()> {
    output.line_stderr("[realtime] stop requested")?;
    Ok(())
}

pub(crate) fn handle_review_start(
    state: &mut AppState,
    output: &mut Output,
    target_description: &str,
) -> Result<()> {
    state.turn_running = true;
    state.activity_started_at = Some(Instant::now());
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
    state.active_turn_id = Some(turn_id.clone());
    state.turn_running = true;
    state.activity_started_at = Some(Instant::now());
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

pub(crate) fn handle_exec_command(
    result: &Value,
    state: &mut AppState,
    output: &mut Output,
    process_id: &str,
    command: &str,
) -> Result<()> {
    let exit_code = result
        .get("exitCode")
        .and_then(Value::as_i64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let buffer = state
        .process_output_buffers
        .remove(process_id)
        .unwrap_or_default();
    let stdout = if buffer.stdout.trim().is_empty() {
        get_string(result, &["stdout"]).unwrap_or("").to_string()
    } else {
        buffer.stdout
    };
    let stderr = if buffer.stderr.trim().is_empty() {
        get_string(result, &["stderr"]).unwrap_or("").to_string()
    } else {
        buffer.stderr
    };
    state.active_exec_process_id = None;
    state.activity_started_at = None;
    state.last_status_line = None;
    output.block_stdout(
        "Local command",
        &render_local_command_completion(command, &exit_code, &stdout, &stderr),
    )?;
    Ok(())
}

pub(crate) fn handle_terminate_exec_command(
    state: &mut AppState,
    output: &mut Output,
    process_id: &str,
) -> Result<()> {
    if state.active_exec_process_id.as_deref() == Some(process_id) {
        state.activity_started_at = None;
        output.line_stderr("[interrupt] local command termination requested")?;
    }
    Ok(())
}
