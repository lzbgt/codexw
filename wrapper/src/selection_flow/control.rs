use anyhow::Result;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;
use crate::state::PendingSelection;

use super::model;
use super::options;

pub(crate) fn open_model_picker(
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    state.pending_selection = Some(PendingSelection::Model);
    Ok(output.block_stdout("Model selection", &model::render_model_picker(cli, state))?)
}

pub(crate) fn open_reasoning_picker(
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    model_id: &str,
) -> Result<()> {
    if model::find_model(state, model_id).is_none() {
        output.line_stderr(format!("[session] unknown model: {model_id}"))?;
        return Ok(());
    }
    state.pending_selection = Some(PendingSelection::ReasoningEffort {
        model_id: model_id.to_string(),
    });
    Ok(output.block_stdout(
        "Reasoning effort",
        &model::render_reasoning_picker(cli, state, model_id),
    )?)
}

pub(crate) fn open_personality_picker(
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    if let Some(model) = crate::model_catalog::effective_model_entry(state, cli)
        && !model.supports_personality
    {
        output.line_stderr(format!(
            "[session] model {} does not support personality overrides",
            model.display_name
        ))?;
        return Ok(());
    }
    state.pending_selection = Some(PendingSelection::Personality);
    Ok(output.block_stdout("Personality", &options::render_personality_picker(state))?)
}

pub(crate) fn open_permissions_picker(
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    state.pending_selection = Some(PendingSelection::Permissions);
    Ok(output.block_stdout(
        "Permissions",
        &options::render_permissions_picker(cli, state),
    )?)
}

pub(crate) fn open_theme_picker(state: &mut AppState, output: &mut Output) -> Result<()> {
    state.pending_selection = Some(PendingSelection::Theme);
    Ok(output.block_stdout("Theme selection", &options::render_theme_picker())?)
}

pub(crate) fn apply_permission_preset(
    preset_id: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    options::apply_permission_preset(preset_id, cli, state, output)
}

pub(crate) fn toggle_fast_mode(state: &mut AppState, output: &mut Output) -> Result<()> {
    options::toggle_fast_mode(state, output)
}

pub(crate) fn apply_theme_choice(
    selector: &str,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    options::apply_theme_choice(selector, state, output)
}

pub(crate) fn apply_model_choice(
    selector: &str,
    effort_override: Option<&str>,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    model::apply_model_choice(selector, effort_override, cli, state, output)
}
