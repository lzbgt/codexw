use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_command_utils::parse_feedback_args;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::send_feedback_upload;
use crate::requests::send_logout_account;
use crate::state::AppState;
use crate::state::summarize_text;

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_handle_session_meta_command(
    command: &str,
    args: &[&str],
    _cli: &Cli,
    _resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "feedback" => {
            let owned = args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
            let Some(parsed) = parse_feedback_args(&owned) else {
                output.line_stderr(
                    "[session] usage: :feedback <bug|bad_result|good_result|safety_check|other> [reason] [--logs|--no-logs]",
                )?;
                return Ok(Some(true));
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
            true
        }
        "logout" => {
            output.line_stderr("[session] logging out")?;
            send_logout_account(writer, state)?;
            true
        }
        "fast"
        | "agent"
        | "multi-agents"
        | "theme"
        | "rollout"
        | "sandbox-add-read-dir"
        | "setup-default-sandbox"
        | "init" => {
            output.line_stderr(format!(
                "[session] /{command} is recognized, but this inline client does not yet implement the native Codex popup/workflow for it"
            ))?;
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}
