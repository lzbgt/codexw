use anyhow::Result;

#[path = "control/interact.rs"]
mod interact;
#[path = "control/services.rs"]
mod services;

use crate::output::Output;
use crate::state::AppState;

pub(super) fn handle_ps_control_action(
    raw_args: &str,
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    if interact::handle_ps_interaction_action(raw_args, args, state, output)? {
        return Ok(true);
    }
    if services::handle_ps_service_action(raw_args, args, state, output)? {
        return Ok(true);
    }
    Ok(false)
}
