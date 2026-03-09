use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::commands_completion_render::quote_if_needed;
use crate::dispatch_command_thread_common::require_idle_turn;
use crate::dispatch_command_thread_draft::handle_thread_draft_command;
use crate::dispatch_command_thread_view::handle_thread_view_command;
use crate::dispatch_command_thread_view::resolve_cached_mention;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;
use crate::state::summarize_text;

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_handle_thread_workspace_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(result) = handle_thread_draft_command(command, args, state, output)? {
        return Ok(Some(result));
    }

    if command == "mention" {
        let query = args.join(" ");
        let query = query.trim();
        if query.is_empty() {
            editor.insert_str("@");
            return Ok(Some(true));
        }
        if let Some(path) = resolve_cached_mention(args, state) {
            let inserted = quote_if_needed(&path);
            editor.insert_str(&format!("{inserted} "));
            output.line_stderr(format!("[mention] inserted {}", summarize_text(&path)))?;
            return Ok(Some(true));
        }
        if query.parse::<usize>().is_ok() {
            output.line_stderr(
                "[session] no cached file match at that index; run /mention <query> first",
            )?;
            return Ok(Some(true));
        }
    }

    if let Some(result) =
        handle_thread_view_command(command, args, resolved_cwd, state, output, writer)?
    {
        return Ok(Some(result));
    }

    let result = match command {
        "clear" => {
            if require_idle_turn(state, output)? {
                output.clear_screen()?;
                output.line_stderr("[thread] creating new thread after clear")?;
                crate::requests::send_thread_start(writer, state, cli, resolved_cwd, None)?;
                true
            } else {
                true
            }
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}
