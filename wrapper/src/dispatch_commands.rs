use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::commands::builtin_help_lines;
use crate::dispatch_command_session;
use crate::dispatch_command_thread;
use crate::dispatch_command_utils;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) use dispatch_command_utils::is_builtin_command;
pub(crate) use dispatch_command_utils::join_prompt;

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
            if let Some(result) = dispatch_command_thread::try_handle_thread_command(
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
            if let Some(result) = dispatch_command_session::try_handle_session_command(
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
