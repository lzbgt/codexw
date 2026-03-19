use anyhow::Result;
use serde_json::Value;

use crate::output::Output;
use crate::requests::PendingRequest;
use crate::state::AppState;

pub(crate) fn handle_runtime_error(
    error: &Value,
    pending: &PendingRequest,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    match pending {
        PendingRequest::StartRealtime { .. } => {
            state.realtime_active = false;
            state.realtime_session_id = None;
            state.realtime_started_at = None;
            state.realtime_prompt = None;
            output.line_stderr("[realtime] request failed")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(error)?
            ))?;
        }
        PendingRequest::AppendRealtimeText { .. } | PendingRequest::StopRealtime => {
            output.line_stderr("[realtime] request failed")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(error)?
            ))?;
        }
        PendingRequest::ExecCommand { process_id, .. } => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.active_exec_process_id = None;
            }
            state.activity_started_at = None;
            state.process_output_buffers.remove(process_id);
            state.last_status_line = None;
            output.line_stderr("[command] failed to start local command")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(error)?
            ))?;
        }
        PendingRequest::TerminateExecCommand { process_id } => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.active_exec_process_id = None;
            }
            state.activity_started_at = None;
            output.line_stderr("[command] failed to terminate local command cleanly")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(error)?
            ))?;
        }
        PendingRequest::InterruptTurn => {
            state.turn_interrupt_requested_at = None;
            output.line_stderr("[interrupt] request failed")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(error)?
            ))?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
