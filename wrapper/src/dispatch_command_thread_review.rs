use std::process::ChildStdin;

use anyhow::Result;
use serde_json::json;

use crate::Cli;
use crate::dispatch_command_thread_common::require_idle_turn;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::send_start_review;
use crate::state::AppState;
use crate::state::summarize_text;
use crate::state::thread_id;

pub(crate) fn try_handle_thread_review_command(
    command: &str,
    args: &[&str],
    _cli: &Cli,
    _resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if command != "review" {
        return Ok(None);
    }

    if require_idle_turn(state, output)? {
        let current_thread_id = thread_id(state)?.to_string();
        let trimmed_args = args.join(" ");
        let trimmed_args = trimmed_args.trim();
        let (target, description) = if trimmed_args.is_empty() {
            (
                json!({"type": "uncommittedChanges"}),
                "current uncommitted changes".to_string(),
            )
        } else {
            (
                json!({"type": "custom", "instructions": trimmed_args}),
                trimmed_args.to_string(),
            )
        };
        output.line_stderr(format!(
            "[review] requesting {}",
            summarize_text(&description)
        ))?;
        send_start_review(writer, state, current_thread_id, target, description)?;
    }

    Ok(Some(true))
}
