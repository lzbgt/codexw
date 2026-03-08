use std::path::Path;
use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_command_thread_common::require_idle_turn;
use crate::dispatch_command_utils::parse_feedback_args;
use crate::editor::LineEditor;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::requests::send_feedback_upload;
use crate::requests::send_logout_account;
use crate::requests::send_thread_start;
use crate::requests::send_turn_start;
use crate::selection_flow::apply_theme_choice;
use crate::selection_flow::open_theme_picker;
use crate::selection_flow::toggle_fast_mode;
use crate::state::AppState;
use crate::state::summarize_text;

pub(crate) const INIT_PROMPT: &str = include_str!("prompt_for_init_command.md");

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_handle_session_meta_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "feedback" => {
            let owned = args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
            let Some(parsed) = parse_feedback_args(&owned) else {
                output.line_stderr(
                    "[session] usage: :feedback <bug|bad_result|good_result|safety_check|other> [reason] [--logs|--no-logs]",
                )?;
                return Ok(Some(true));
            };
            let current_thread = state.thread_id.clone();
            output.line_stderr(format!(
                "[feedback] submitting {} feedback",
                summarize_text(&parsed.classification)
            ))?;
            send_feedback_upload(
                writer,
                state,
                parsed.classification,
                parsed.reason,
                current_thread,
                parsed.include_logs,
            )?;
            true
        }
        "logout" => {
            output.line_stderr("[session] logging out")?;
            send_logout_account(writer, state)?;
            true
        }
        "fast" => {
            toggle_fast_mode(state, output)?;
            true
        }
        "theme" => {
            if args.is_empty() {
                open_theme_picker(state, output)?;
            } else {
                apply_theme_choice(&args.join(" "), state, output)?;
            }
            true
        }
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
        "agent" | "multi-agents" | "sandbox-add-read-dir" | "setup-default-sandbox" => {
            output.line_stderr(format!(
                "[session] /{command} is recognized, but this inline client does not yet implement the native Codex popup/workflow for it"
            ))?;
            true
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
