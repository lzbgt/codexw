use std::process::ChildStdin;

use anyhow::Result;

use crate::dispatch_command_utils::parse_feedback_args;
use crate::output::Output;
use crate::requests::send_feedback_upload;
use crate::requests::send_logout_account;
use crate::state::AppState;
use crate::state::summarize_text;

pub(crate) fn handle_feedback_command(
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let owned = args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    let Some(parsed) = parse_feedback_args(&owned) else {
        output.line_stderr(
            "[session] usage: :feedback <bug|bad_result|good_result|safety_check|other> [reason] [--logs|--no-logs]",
        )?;
        return Ok(true);
    };
    let current_thread = state.thread_id.clone();
    output.line_stderr(format!(
        "[feedback] submitting {} feedback",
        summarize_text(&parsed.classification)
    ))?;
    send_feedback_upload(
        writer,
        state,
        parsed.classification,
        parsed.reason,
        current_thread,
        parsed.include_logs,
    )?;
    Ok(true)
}

pub(crate) fn handle_logout_command(
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    output.line_stderr("[session] logging out")?;
    send_logout_account(writer, state)?;
    Ok(true)
}
