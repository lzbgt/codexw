#[path = "interact/jobs.rs"]
mod jobs;
#[path = "interact/services.rs"]
mod services;

use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;

pub(super) fn handle_ps_interaction_action(
    raw_args: &str,
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    jobs::handle_ps_interaction_action(raw_args, args, state, output)
}
