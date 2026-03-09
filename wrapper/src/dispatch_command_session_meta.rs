use std::path::Path;
use std::process::ChildStdin;

use anyhow::Result;

#[path = "dispatch_command_session_meta/account.rs"]
mod account;
#[path = "dispatch_command_session_meta/sandbox.rs"]
mod sandbox;
#[path = "dispatch_command_session_meta/session.rs"]
mod session;

use crate::Cli;
use crate::dispatch_command_thread_common::require_idle_turn;
use crate::editor::LineEditor;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::requests::send_list_agent_threads;
use crate::requests::send_thread_start;
use crate::requests::send_turn_start;
use crate::state::AppState;
use account::handle_feedback_command;
use account::handle_logout_command;
use sandbox::handle_sandbox_add_read_dir_command;
use sandbox::handle_setup_default_sandbox_command;
use session::handle_fast_command;
use session::handle_theme_command;

pub(crate) const INIT_PROMPT: &str = include_str!("prompt_for_init_command.md");

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_handle_session_meta_command(
    command: &str,
    args: &[&str],
    raw_args: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "feedback" => handle_feedback_command(args, state, output, writer)?,
        "logout" => handle_logout_command(state, output, writer)?,
        "fast" => handle_fast_command(state, output)?,
        "theme" => handle_theme_command(args, state, output)?,
        "init" => {
            if !args.is_empty() {
                output.line_stderr("[session] usage: :init")?;
                return Ok(Some(true));
            }
            handle_init_command(cli, resolved_cwd, state, output, writer)?
        }
        "rollout" => {
            if !args.is_empty() {
                output.line_stderr("[session] usage: :rollout")?;
                return Ok(Some(true));
            }
            output.line_stderr(current_rollout_message(state))?;
            true
        }
        "agent" | "multi-agents" => {
            if !args.is_empty() {
                output.line_stderr(format!("[session] usage: :{command}"))?;
                return Ok(Some(true));
            }
            output.line_stderr("[session] loading recent agent threads")?;
            send_list_agent_threads(writer, state, Some(resolved_cwd))?;
            true
        }
        "setup-default-sandbox" => {
            handle_setup_default_sandbox_command(args, resolved_cwd, state, output, writer)?
        }
        "sandbox-add-read-dir" => {
            handle_sandbox_add_read_dir_command(raw_args, cli, resolved_cwd, state, output)?
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}

fn handle_init_command(
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let init_target = Path::new(resolved_cwd).join("AGENTS.md");
    if init_target.exists() {
        output.line_stderr(
            "AGENTS.md already exists here. Skipping /init to avoid overwriting it.",
        )?;
        return Ok(true);
    }
    if !require_idle_turn(state, output)? {
        return Ok(true);
    }

    let prompt = INIT_PROMPT.trim_end().to_string();
    if let Some(thread_id) = state.thread_id.clone() {
        let submission = build_turn_input(
            &prompt,
            resolved_cwd,
            &[],
            &[],
            &state.apps,
            &state.plugins,
            &state.skills,
        );
        if submission.items.is_empty() {
            output.line_stderr("[session] /init prompt produced no input")?;
            return Ok(true);
        }
        output.line_stderr("[session] requesting AGENTS.md draft")?;
        send_turn_start(
            writer,
            state,
            cli,
            resolved_cwd,
            thread_id,
            submission,
            false,
        )?;
    } else {
        output.line_stderr("[thread] creating thread for /init")?;
        send_thread_start(writer, state, cli, resolved_cwd, Some(prompt))?;
    }
    Ok(true)
}

pub(crate) fn current_rollout_message(state: &AppState) -> String {
    state
        .current_rollout_path
        .as_ref()
        .map(|path| format!("Current rollout path: {}", path.display()))
        .unwrap_or_else(|| "Rollout path is not available yet.".to_string())
}
