use anyhow::Result;
use serde_json::Value;

use crate::output::Output;
use crate::response_local_command::handle_exec_command;
use crate::response_local_command::handle_terminate_exec_command;
use crate::response_realtime_activity::handle_realtime_append;
use crate::response_realtime_activity::handle_realtime_start;
use crate::response_realtime_activity::handle_realtime_stop;
use crate::response_turn_activity::handle_review_start;
use crate::response_turn_activity::handle_turn_interrupt;
use crate::response_turn_activity::handle_turn_start;
use crate::response_turn_activity::handle_turn_steer;
use crate::state::AppState;

pub(crate) fn handle_thread_runtime_response(
    pending: &crate::requests::PendingRequest,
    result: &Value,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    match pending {
        crate::requests::PendingRequest::StartRealtime { prompt } => {
            handle_realtime_start(state, output, prompt)?;
        }
        crate::requests::PendingRequest::AppendRealtimeText { text } => {
            handle_realtime_append(output, text)?;
        }
        crate::requests::PendingRequest::StopRealtime => {
            handle_realtime_stop(output)?;
        }
        crate::requests::PendingRequest::StartReview { target_description } => {
            handle_review_start(state, output, target_description)?;
        }
        crate::requests::PendingRequest::StartTurn { auto_generated } => {
            handle_turn_start(result, state, output, *auto_generated)?;
        }
        crate::requests::PendingRequest::SteerTurn { display_text } => {
            handle_turn_steer(result, state, output, display_text)?;
        }
        crate::requests::PendingRequest::InterruptTurn => {
            handle_turn_interrupt(output)?;
        }
        crate::requests::PendingRequest::ExecCommand {
            process_id,
            command,
        } => {
            handle_exec_command(result, state, output, process_id, command)?;
        }
        crate::requests::PendingRequest::TerminateExecCommand { process_id } => {
            handle_terminate_exec_command(state, output, process_id)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
