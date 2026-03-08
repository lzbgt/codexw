use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_commands::handle_command;
use crate::dispatch_commands::is_builtin_command;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::send_command_exec;
use crate::selection_flow::handle_pending_selection;
use crate::state::AppState;
use crate::state::emit_status_line;
use crate::state::summarize_text;

pub(crate) fn try_handle_prefixed_submission(
    trimmed: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if let Some(command) = trimmed.strip_prefix(':') {
        return handle_command(command, cli, resolved_cwd, state, editor, output, writer).map(Some);
    }
    if let Some(command) = trimmed.strip_prefix('/')
        && is_builtin_command(command)
    {
        return handle_command(command, cli, resolved_cwd, state, editor, output, writer).map(Some);
    }

    if let Some(command) = trimmed.strip_prefix('!') {
        if state.turn_running {
            output.line_stderr(
                "[session] wait for the active turn to finish before running a local command",
            )?;
            return Ok(Some(true));
        }
        if state.active_exec_process_id.is_some() {
            output.line_stderr("[session] a local command is already running")?;
            return Ok(Some(true));
        }
        let command = command.trim();
        if command.is_empty() {
            output.line_stderr("[session] usage: !<shell command>")?;
            return Ok(Some(true));
        }
        emit_status_line(
            output,
            state,
            format!("running local command: {}", summarize_text(command)),
        )?;
        send_command_exec(writer, state, cli, resolved_cwd, command.to_string())?;
        return Ok(Some(true));
    }

    if state.pending_selection.is_some() {
        return handle_pending_selection(trimmed, cli, state, output).map(Some);
    }

    if state.startup_resume_picker && state.thread_id.is_none() {
        if is_builtin_command(trimmed) {
            return handle_command(trimmed, cli, resolved_cwd, state, editor, output, writer)
                .map(Some);
        }
        if !trimmed.contains(char::is_whitespace) {
            return handle_command(
                &format!("resume {trimmed}"),
                cli,
                resolved_cwd,
                state,
                editor,
                output,
                writer,
            )
            .map(Some);
        }
        output.line_stderr(
            "[session] enter a listed number or thread id, or use /resume <n> or /new",
        )?;
        return Ok(Some(true));
    }

    Ok(None)
}
