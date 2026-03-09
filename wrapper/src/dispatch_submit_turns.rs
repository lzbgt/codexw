use std::process::ChildStdin;

use anyhow::Context;
use anyhow::Result;

use crate::Cli;
use crate::input::build_turn_input;
use crate::requests::send_turn_start;
use crate::requests::send_turn_steer;
use crate::state::AppState;
use crate::state::thread_id;

pub(crate) fn submit_turn_input(
    trimmed: &str,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    writer: &mut ChildStdin,
) -> Result<bool> {
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
