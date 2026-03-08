use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;
use crate::transcript_completion_render::render_local_command_completion;

pub(crate) fn handle_exec_command(
    result: &Value,
    cli: &Cli,
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
        &render_local_command_completion(
            command,
            &exit_code,
            &stdout,
            &stderr,
            cli.verbose_events || cli.raw_json,
        ),
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
