use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;

const ACTIVE_TURN_MESSAGE: &str =
    "[session] wait for the current turn to finish or interrupt it first";

pub(crate) fn require_idle_turn(state: &AppState, output: &mut Output) -> Result<bool> {
    if state.turn_running {
        output.line_stderr(ACTIVE_TURN_MESSAGE)?;
        Ok(false)
    } else {
        Ok(true)
    }
}

pub(crate) fn resolve_cached_thread_reference(
    arg: &str,
    state: &AppState,
    output: &mut Output,
) -> Result<Option<String>> {
    if let Ok(index) = arg.parse::<usize>() {
        match state.last_listed_thread_ids.get(index.saturating_sub(1)) {
            Some(thread_id) => Ok(Some(thread_id.clone())),
            None => {
                output.line_stderr(
                    "[session] no cached thread at that index; run /threads or /resume first",
                )?;
                Ok(None)
            }
        }
    } else {
        Ok(Some(arg.to_string()))
    }
}
