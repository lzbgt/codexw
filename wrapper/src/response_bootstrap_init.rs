use std::process::ChildStdin;

use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::collaboration::CollaborationModeAction;
use crate::model_session::ModelsAction;
use crate::output::Output;
use crate::requests::send_initialized;
use crate::requests::send_load_account;
use crate::requests::send_load_apps;
use crate::requests::send_load_collaboration_modes;
use crate::requests::send_load_models;
use crate::requests::send_load_rate_limits;
use crate::requests::send_load_skills;
use crate::runtime_process::StartMode;
use crate::state::AppState;
use crate::state::get_string;
use crate::state::summarize_text;

pub(crate) fn handle_initialize_success(
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    start_after_initialize: &mut Option<StartMode>,
) -> Result<()> {
    send_initialized(writer)?;
    output.line_stderr("[session] connected")?;
    if let Some(start_mode) = start_after_initialize.take() {
        match start_mode.resume_thread_id {
            Some(thread_id) => {
                output.line_stderr(format!("[thread] resume {thread_id}"))?;
                crate::requests::send_thread_resume(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    thread_id,
                    start_mode.initial_prompt,
                )?
            }
            None => {
                output.line_stderr("[thread] create")?;
                crate::requests::send_thread_start(
                    writer,
                    state,
                    cli,
                    resolved_cwd,
                    start_mode.initial_prompt,
                )?
            }
        }
    }
    send_load_apps(writer, state)?;
    send_load_skills(writer, state, resolved_cwd)?;
    send_load_models(writer, state, ModelsAction::CacheOnly)?;
    send_load_collaboration_modes(writer, state, CollaborationModeAction::CacheOnly)?;
    send_load_account(writer, state)?;
    send_load_rate_limits(writer, state)?;
    Ok(())
}

pub(crate) fn handle_logout_success(
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    state.account_info = None;
    state.rate_limits = None;
    output.line_stderr("[session] logged out")?;
    send_load_account(writer, state)?;
    send_load_rate_limits(writer, state)?;
    Ok(())
}

pub(crate) fn handle_feedback_success(
    result: &Value,
    classification: &str,
    output: &mut Output,
) -> Result<()> {
    let tracking_thread = get_string(result, &["threadId"]).unwrap_or("-");
    output.line_stderr(format!(
        "[feedback] submitted {} feedback; tracking thread {}",
        summarize_text(classification),
        tracking_thread
    ))?;
    Ok(())
}
