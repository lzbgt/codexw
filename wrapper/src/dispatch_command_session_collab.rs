use std::process::ChildStdin;

use anyhow::Result;

use crate::collaboration_apply::CollaborationModeAction;
use crate::output::Output;
use crate::requests::send_load_collaboration_modes;
use crate::state::AppState;

pub(crate) fn handle_collab_command(
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    if args.is_empty() {
        send_load_collaboration_modes(writer, state, CollaborationModeAction::ShowList)?;
    } else if state.turn_running {
        output.line_stderr("[session] cannot switch collaboration mode while a turn is running")?;
    } else {
        let selector = args.join(" ");
        send_load_collaboration_modes(writer, state, CollaborationModeAction::SetMode(selector))?;
    }
    Ok(true)
}

pub(crate) fn handle_plan_command(
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    if state.turn_running {
        output.line_stderr("[session] cannot switch collaboration mode while a turn is running")?;
    } else {
        send_load_collaboration_modes(writer, state, CollaborationModeAction::TogglePlan)?;
    }
    Ok(true)
}
