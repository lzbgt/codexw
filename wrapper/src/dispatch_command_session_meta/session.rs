use anyhow::Result;

use crate::output::Output;
use crate::selection_flow::apply_theme_choice;
use crate::selection_flow::open_theme_picker;
use crate::selection_flow::toggle_fast_mode;
use crate::state::AppState;

pub(crate) fn handle_fast_command(state: &mut AppState, output: &mut Output) -> Result<bool> {
    toggle_fast_mode(state, output)?;
    Ok(true)
}

pub(crate) fn handle_theme_command(
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    if args.is_empty() {
        open_theme_picker(state, output)?;
    } else {
        apply_theme_choice(&args.join(" "), state, output)?;
    }
    Ok(true)
}
