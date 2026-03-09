use std::process::ChildStdin;

use anyhow::Result;

#[path = "dispatch_commands/session.rs"]
mod session;
#[path = "dispatch_commands/workspace.rs"]
mod workspace;

use crate::Cli;
use crate::commands_catalog::builtin_help_lines;
use crate::dispatch_command_session_catalog_lists::try_handle_session_catalog_list_command;
use crate::dispatch_command_session_catalog_models::try_handle_session_catalog_model_command;
use crate::dispatch_command_session_meta::try_handle_session_meta_command;
use crate::dispatch_command_session_status::try_handle_session_status_command;
use crate::dispatch_command_thread_control::try_handle_thread_control_command;
use crate::dispatch_command_thread_navigation_identity::try_handle_thread_identity_navigation;
use crate::dispatch_command_thread_navigation_session::try_handle_thread_session_navigation;
use crate::dispatch_command_thread_review::try_handle_thread_review_command;
use crate::dispatch_command_utils;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) use dispatch_command_utils::is_builtin_command;
use session::try_handle_session_builtin_command;
pub(crate) use workspace::try_handle_thread_workspace_command;

pub(crate) fn handle_command(
    command_line: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let trimmed = command_line.trim();
    let Some(command) = trimmed.split_whitespace().next() else {
        output.line_stderr("[session] empty command")?;
        return Ok(true);
    };
    let raw_args = trimmed
        .strip_prefix(command)
        .map(str::trim_start)
        .unwrap_or_default();
    let args = raw_args.split_whitespace().collect::<Vec<_>>();

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
            if let Some(result) = try_handle_thread_session_navigation(
                command,
                &args,
                cli,
                resolved_cwd,
                state,
                output,
                writer,
            )? {
                return Ok(result);
            }
            if let Some(result) = try_handle_thread_identity_navigation(
                command,
                &args,
                cli,
                resolved_cwd,
                state,
                output,
                writer,
            )? {
                return Ok(result);
            }
            if let Some(result) = try_handle_thread_review_command(
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
            if let Some(result) = try_handle_thread_control_command(
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
            if let Some(result) = try_handle_thread_workspace_command(
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
            if let Some(result) = try_handle_session_builtin_command(
                command, raw_args, &args, cli, state, output, writer,
            )? {
                return Ok(result);
            }
            if let Some(result) = try_handle_session_meta_command(
                command,
                &args,
                raw_args,
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
