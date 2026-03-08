use anyhow::Result;
use serde_json::Value;

use crate::collaboration_apply::CollaborationModeAction;
use crate::output::Output;
use crate::requests::PendingRequest;
use crate::state::AppState;
use crate::state::summarize_text;

pub(crate) fn handle_session_error(
    error: &Value,
    pending: &PendingRequest,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    match pending {
        PendingRequest::LoadRateLimits => {
            output.line_stderr("[session] rate limits unavailable for the current account")?;
        }
        PendingRequest::LoadAccount => {
            output.line_stderr("[session] account details unavailable from app-server")?;
        }
        PendingRequest::LoadModels { .. } => {
            output.line_stderr("[session] model metadata unavailable from app-server")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(error)?
            ))?;
        }
        PendingRequest::LoadCollaborationModes { action } => {
            if !matches!(action, CollaborationModeAction::CacheOnly) {
                output.line_stderr(
                    "[session] collaboration modes are unavailable from this app-server build",
                )?;
                output.line_stderr(format!(
                    "[server-error] {}",
                    serde_json::to_string_pretty(error)?
                ))?;
            }
        }
        PendingRequest::LogoutAccount => {
            output.line_stderr("[session] logout failed")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(error)?
            ))?;
        }
        PendingRequest::UploadFeedback { classification } => {
            output.line_stderr(format!(
                "[feedback] failed to submit {} feedback",
                summarize_text(classification)
            ))?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(error)?
            ))?;
        }
        PendingRequest::StartThread { .. }
        | PendingRequest::ResumeThread { .. }
        | PendingRequest::ForkThread { .. } => {
            state.pending_thread_switch = false;
            output.line_stderr("[thread] failed to switch threads")?;
            output.line_stderr(format!(
                "[server-error] {}",
                serde_json::to_string_pretty(error)?
            ))?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
