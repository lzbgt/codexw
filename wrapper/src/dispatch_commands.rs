use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::commands_metadata::builtin_help_lines;
use crate::dispatch_command_session_catalog_lists::try_handle_session_catalog_list_command;
use crate::dispatch_command_session_catalog_models::try_handle_session_catalog_model_command;
use crate::dispatch_command_session_collab::handle_collab_command;
use crate::dispatch_command_session_collab::handle_plan_command;
use crate::dispatch_command_session_meta::try_handle_session_meta_command;
use crate::dispatch_command_session_ps::handle_ps_command;
use crate::dispatch_command_session_realtime::handle_realtime_command;
use crate::dispatch_command_session_status::try_handle_session_status_command;
use crate::dispatch_command_thread_actions;
use crate::dispatch_command_thread_navigation;
use crate::dispatch_command_thread_workspace;
use crate::dispatch_command_utils;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) use dispatch_command_utils::is_builtin_command;

pub(crate) fn handle_command(
    command_line: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let mut parts = command_line.split_whitespace();
    let Some(command) = parts.next() else {
        output.line_stderr("[session] empty command")?;
        return Ok(true);
    };
    let args = parts.collect::<Vec<_>>();

    match command {
        "help" | "h" => {
            for line in builtin_help_lines() {
                output.line_stderr(line)?;
            }
            output.line_stderr(
                "!<command>           run a local shell command via app-server command/exec",
            )?;
            Ok(true)
        }
        "quit" | "q" | "exit" => Ok(false),
        _ => {
            if let Some(result) =
                dispatch_command_thread_navigation::try_handle_thread_navigation_command(
                    command,
                    &args,
                    cli,
                    resolved_cwd,
                    state,
                    editor,
                    output,
                    writer,
                )?
            {
                return Ok(result);
            }
            if let Some(result) = dispatch_command_thread_actions::try_handle_thread_action_command(
                command,
                &args,
                cli,
                resolved_cwd,
                state,
                editor,
                output,
                writer,
            )? {
                return Ok(result);
            }
            if let Some(result) =
                dispatch_command_thread_workspace::try_handle_thread_workspace_command(
                    command,
                    &args,
                    cli,
                    resolved_cwd,
                    state,
                    editor,
                    output,
                    writer,
                )?
            {
                return Ok(result);
            }
            if let Some(result) =
                try_handle_session_catalog_list_command(command, state, output, writer)?
            {
                return Ok(result);
            }
            if let Some(result) = try_handle_session_catalog_model_command(
                command, &args, cli, state, output, writer,
            )? {
                return Ok(result);
            }
            if let Some(result) = try_handle_session_status_command(
                command,
                &args,
                cli,
                resolved_cwd,
                state,
                editor,
                output,
                writer,
            )? {
                return Ok(result);
            }
            if let Some(result) = match command {
                "auto" => {
                    let Some(mode) = args.first() else {
                        output.line_stderr("[session] usage: :auto on|off")?;
                        return Ok(true);
                    };
                    state.auto_continue = match *mode {
                        "on" => true,
                        "off" => false,
                        _ => {
                            output.line_stderr("[session] usage: :auto on|off")?;
                            return Ok(true);
                        }
                    };
                    output.line_stderr(format!(
                        "[auto] {}",
                        if state.auto_continue {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    ))?;
                    Some(true)
                }
                "collab" => Some(handle_collab_command(&args, state, output, writer)?),
                "plan" => Some(handle_plan_command(state, output, writer)?),
                "realtime" => handle_realtime_command(&args, cli, state, output, writer)?,
                "ps" => Some(handle_ps_command(&args, cli, state, output, writer)?),
                _ => None,
            } {
                return Ok(result);
            }
            if let Some(result) = try_handle_session_meta_command(
                command,
                &args,
                cli,
                resolved_cwd,
                state,
                editor,
                output,
                writer,
            )? {
                return Ok(result);
            }
            output.line_stderr(format!("[session] unknown command: {command}"))?;
            Ok(true)
        }
    }
}
