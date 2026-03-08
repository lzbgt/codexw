use std::process::ChildStdin;

use anyhow::Context;
use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::history_render::render_resumed_history;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::requests::send_turn_start;
use crate::state::AppState;
use crate::state::get_string;

fn send_initial_thread_prompt(
    text: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    writer: &mut ChildStdin,
    thread_id: String,
) -> Result<()> {
    let submission = build_turn_input(
        text,
        resolved_cwd,
        &[],
        &[],
        &state.apps,
        &state.plugins,
        &state.skills,
    );
    send_turn_start(
        writer,
        state,
        cli,
        resolved_cwd,
        thread_id,
        submission,
        false,
    )
}

pub(crate) fn handle_loaded_thread(
    result: &Value,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    initial_prompt: Option<&str>,
    status_label: &str,
    thread_field_context: &'static str,
    render_history: bool,
) -> Result<()> {
    state.pending_thread_switch = false;
    state.reset_thread_context();
    let thread_id = get_string(result, &["thread", "id"])
        .context(thread_field_context)?
        .to_string();
    state.thread_id = Some(thread_id.clone());
    output.line_stderr(format!("[thread] {status_label} {thread_id}"))?;
    if render_history {
        render_resumed_history(result, state, output)?;
    }
    if let Some(text) = initial_prompt {
        send_initial_thread_prompt(text, cli, resolved_cwd, state, writer, thread_id)?;
    }
    Ok(())
}
