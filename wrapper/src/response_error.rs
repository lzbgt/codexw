use anyhow::Result;
use serde_json::Value;

use crate::collaboration::CollaborationModeAction;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::state::AppState;
use crate::state::summarize_text;

pub(crate) fn handle_response_error_impl(
    error: Value,
    pending: Option<PendingRequest>,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    match pending {
        Some(PendingRequest::LoadRateLimits) => {
            output.line_stderr("[session] rate limits unavailable for the current account")?;
        }
        Some(PendingRequest::LoadAccount) => {
            output.line_stderr("[session] account details unavailable from app-server")?;
        }
        Some(PendingRequest::LoadModels { .. }) => {
            output.line_stderr("[session] model metadata unavailable from app-server")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::LoadCollaborationModes { action }) => {
            if !matches!(action, CollaborationModeAction::CacheOnly) {
                output.line_stderr(
                    "[session] collaboration modes are unavailable from this app-server build",
                )?;
                output.line_stderr(format!(
                    "[server-error] {}",
                    serde_json::to_string_pretty(&error)?
                ))?;
            }
        }
        Some(PendingRequest::LogoutAccount) => {
            output.line_stderr("[session] logout failed")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::StartRealtime { .. })
        | Some(PendingRequest::AppendRealtimeText { .. })
        | Some(PendingRequest::StopRealtime) => {
            output.line_stderr("[realtime] request failed")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::UploadFeedback { classification }) => {
            output.line_stderr(format!(
                "[feedback] failed to submit {} feedback",
                summarize_text(&classification)
            ))?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::StartThread { .. })
        | Some(PendingRequest::ResumeThread { .. })
        | Some(PendingRequest::ForkThread { .. }) => {
            state.pending_thread_switch = false;
            output.line_stderr("[thread] failed to switch threads")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::ExecCommand { process_id, .. }) => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.active_exec_process_id = None;
            }
            state.activity_started_at = None;
            state.process_output_buffers.remove(&process_id);
            state.last_status_line = None;
            output.line_stderr("[command] failed to start local command")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        Some(PendingRequest::TerminateExecCommand { process_id }) => {
            if state.active_exec_process_id.as_deref() == Some(process_id.as_str()) {
                state.active_exec_process_id = None;
            }
            state.activity_started_at = None;
            output.line_stderr("[command] failed to terminate local command cleanly")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
        _ => {
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(&error)?
            ))?;
        }
    }
    Ok(())
}
