use anyhow::Result;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;
use crate::state::PendingSelection;

use super::model;
use super::options;

pub(crate) fn pending_selection_status(selection: &PendingSelection) -> String {
    match selection {
        PendingSelection::Model => {
            "model picker | enter a number or model id | /cancel to dismiss".to_string()
        }
        PendingSelection::ReasoningEffort { model_id } => {
            format!("reasoning picker | {model_id} | enter a number or effort | /cancel to dismiss")
        }
        PendingSelection::Personality => {
            "personality picker | enter a number or label | /cancel to dismiss".to_string()
        }
        PendingSelection::Permissions => {
            "permissions picker | enter a number or preset id | /cancel to dismiss".to_string()
        }
        PendingSelection::Theme => {
            "theme picker | enter a number or theme name | /cancel to dismiss".to_string()
        }
    }
}

pub(crate) fn handle_pending_selection(
    trimmed: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let Some(selection) = state.pending_selection.clone() else {
        return Ok(false);
    };
    if matches!(trimmed, "/cancel" | ":cancel" | "cancel") {
        state.pending_selection = None;
        output.line_stderr("[session] selection cancelled")?;
        return Ok(true);
    }

    match selection {
        PendingSelection::Model => model::handle_model_picker_input(trimmed, cli, state, output),
        PendingSelection::ReasoningEffort { model_id } => {
            model::handle_reasoning_picker_input(trimmed, state, output, &model_id)
        }
        PendingSelection::Personality => {
            options::handle_personality_picker_input(trimmed, cli, state, output)
        }
        PendingSelection::Permissions => {
            options::handle_permissions_picker_input(trimmed, cli, state, output)
        }
        PendingSelection::Theme => options::handle_theme_picker_input(trimmed, state, output),
    }
}
