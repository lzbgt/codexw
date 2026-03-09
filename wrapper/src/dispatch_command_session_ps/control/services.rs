#[path = "services/dependencies.rs"]
mod dependencies;
#[path = "services/metadata.rs"]
mod metadata;

use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;

pub(super) fn handle_ps_service_action(
    raw_args: &str,
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    if metadata::handle_ps_service_metadata_action(raw_args, args, state, output)? {
        return Ok(true);
    }
    if dependencies::handle_ps_service_dependency_action(args, state, output)? {
        return Ok(true);
    }
    Ok(false)
}
