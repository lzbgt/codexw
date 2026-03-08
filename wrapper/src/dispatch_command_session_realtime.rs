use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::requests::send_thread_realtime_append_text;
use crate::requests::send_thread_realtime_start;
use crate::requests::send_thread_realtime_stop;
use crate::state::AppState;
use crate::state::thread_id;

pub(crate) fn handle_realtime_command(
    args: &[&str],
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if cli.no_experimental_api {
        output.line_stderr(
            "[session] /realtime requires experimental API support; restart without --no-experimental-api",
        )?;
        return Ok(Some(true));
    }
    let Some(thread_id) = state.thread_id.clone() else {
        output.line_stderr("[session] start or resume a thread before using /realtime")?;
        return Ok(Some(true));
    };
    if args.is_empty() || matches!(args[0], "status" | "show") {
        return Ok(None);
    }
    match args[0] {
        "start" => {
            if state.turn_running {
                output.line_stderr("[session] cannot start realtime while a turn is running")?;
            } else if state.realtime_active {
                output.line_stderr(
                    "[session] realtime is already active; use /realtime stop first",
                )?;
                output.block_stdout(
                    "Realtime",
                    &crate::session_realtime::render_realtime_status(state),
                )?;
            } else {
                let prompt = if args.len() > 1 {
                    args[1..].join(" ")
                } else {
                    "Text-only experimental realtime session for this thread.".to_string()
                };
                send_thread_realtime_start(writer, state, thread_id, prompt)?;
            }
        }
        "send" | "append" => {
            if !state.realtime_active {
                output
                    .line_stderr("[session] realtime is not active; use /realtime start first")?;
            } else if args.len() < 2 {
                output.line_stderr("[session] usage: /realtime send <text>")?;
            } else {
                send_thread_realtime_append_text(writer, state, thread_id, args[1..].join(" "))?;
            }
        }
        "stop" => {
            if !state.realtime_active {
                output.line_stderr("[session] realtime is not active")?;
            } else {
                send_thread_realtime_stop(writer, state, thread_id)?;
            }
        }
        other => {
            output.line_stderr(format!("[session] unknown realtime action: {other}"))?;
            output.block_stdout(
                "Realtime",
                &crate::session_realtime::render_realtime_status(state),
            )?;
        }
    }
    Ok(Some(true))
}

pub(crate) fn handle_ps_command(
    args: &[&str],
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let action = args.first().copied();
    if matches!(action, Some("clean")) {
        if cli.no_experimental_api {
            output.line_stderr(
                "[thread] /ps clean requires experimental API support; restart without --no-experimental-api",
            )?;
        } else {
            let current_thread_id = thread_id(state)?.to_string();
            output.line_stderr("[thread] cleaning background terminals")?;
            send_clean_background_terminals(writer, state, current_thread_id)?;
        }
    } else {
        output.line_stderr(
            "[session] app-server does not expose background-terminal listing like the native TUI; use /ps clean to stop all background terminals for this thread",
        )?;
    }
    Ok(true)
}
