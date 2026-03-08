use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::commands_catalog::builtin_help_lines;
use crate::commands_completion_render::quote_if_needed;
use crate::dispatch_command_session_catalog_lists::try_handle_session_catalog_list_command;
use crate::dispatch_command_session_catalog_models::try_handle_session_catalog_model_command;
use crate::dispatch_command_session_collab::handle_collab_command;
use crate::dispatch_command_session_collab::handle_plan_command;
use crate::dispatch_command_session_meta::try_handle_session_meta_command;
use crate::dispatch_command_session_ps::handle_ps_command;
use crate::dispatch_command_session_realtime::handle_realtime_command;
use crate::dispatch_command_session_status::try_handle_session_status_command;
use crate::dispatch_command_thread_common::require_idle_turn;
use crate::dispatch_command_thread_control::try_handle_thread_control_command;
use crate::dispatch_command_thread_draft::handle_thread_draft_command;
use crate::dispatch_command_thread_navigation_identity::try_handle_thread_identity_navigation;
use crate::dispatch_command_thread_navigation_session::try_handle_thread_session_navigation;
use crate::dispatch_command_thread_review::try_handle_thread_review_command;
use crate::dispatch_command_thread_view::handle_thread_view_command;
use crate::dispatch_command_thread_view::resolve_cached_mention;
use crate::dispatch_command_utils;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;
use crate::state::summarize_text;

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
                "ps" => Some(handle_ps_command(
                    raw_args, &args, cli, state, output, writer,
                )?),
                _ => None,
            } {
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

#[allow(clippy::too_many_arguments)]
fn try_handle_thread_workspace_command(
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
