use std::process::ChildStdin;

use anyhow::Context;
use anyhow::Result;

use crate::Cli;
use crate::dispatch::handle_command;
use crate::dispatch::is_builtin_command;
use crate::editor::LineEditor;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::requests::send_command_exec;
use crate::requests::send_turn_start;
use crate::requests::send_turn_steer;
use crate::state::AppState;
use crate::state::emit_status_line;
use crate::state::summarize_text;
use crate::state::thread_id;

pub(crate) fn handle_user_input(
    line: String,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(true);
    }

    if let Some(command) = trimmed.strip_prefix(':') {
        return handle_command(command, cli, resolved_cwd, state, editor, output, writer);
    }
    if let Some(command) = trimmed.strip_prefix('/')
        && is_builtin_command(command)
    {
        return handle_command(command, cli, resolved_cwd, state, editor, output, writer);
    }

    if let Some(command) = trimmed.strip_prefix('!') {
        if state.turn_running {
            output.line_stderr(
                "[session] wait for the active turn to finish before running a local command",
            )?;
            return Ok(true);
        }
        if state.active_exec_process_id.is_some() {
            output.line_stderr("[session] a local command is already running")?;
            return Ok(true);
        }
        let command = command.trim();
        if command.is_empty() {
            output.line_stderr("[session] usage: !<shell command>")?;
            return Ok(true);
        }
        emit_status_line(
            output,
            state,
            format!("running local command: {}", summarize_text(command)),
        )?;
        send_command_exec(writer, state, cli, resolved_cwd, command.to_string())?;
        return Ok(true);
    }

    let (local_images, remote_images) = state.take_pending_attachments();
    let submission = build_turn_input(
        trimmed,
        resolved_cwd,
        &local_images,
        &remote_images,
        &state.apps,
        &state.plugins,
        &state.skills,
    );
    if submission.items.is_empty() {
        output.line_stderr("[session] nothing to submit")?;
        return Ok(true);
    }

    let thread_id = thread_id(state)?.to_string();
    if state.turn_running {
        let turn_id = state
            .active_turn_id
            .clone()
            .context("turn is marked running but active turn id is missing")?;
        send_turn_steer(writer, state, thread_id, turn_id, submission)?;
    } else {
        send_turn_start(
            writer,
            state,
            cli,
            resolved_cwd,
            thread_id,
            submission,
            false,
        )?;
    }
    Ok(true)
}
