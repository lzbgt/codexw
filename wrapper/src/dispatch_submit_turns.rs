use std::process::ChildStdin;

use anyhow::Context;
use anyhow::Result;

use crate::Cli;
use crate::input::build_turn_input;
use crate::output::Output;
use crate::requests::send_turn_interrupt;
use crate::requests::send_turn_start;
use crate::requests::send_turn_steer;
use crate::state::AppState;
use crate::state::thread_id;

pub(crate) fn submit_turn_input(
    trimmed: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    if state.turn_running && state.stalled_turn_idle_for().is_some() {
        let thread_id = thread_id(state)?.to_string();
        let turn_id = state
            .active_turn_id
            .clone()
            .context("quiet turn is marked running but active turn id is missing")?;
        state.stage_resume_prompt(trimmed.to_string());
        state.push_conversation_message("user", trimmed);
        output.line_stderr(
            "[self-supervision] quiet turn detected; queued the new prompt for resume and requested an interrupt instead of steering the current turn",
        )?;
        send_turn_interrupt(writer, state, thread_id, turn_id)?;
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
        return Ok(false);
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

    state.push_conversation_message("user", trimmed);

    Ok(true)
}
