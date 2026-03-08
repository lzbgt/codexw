use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;

#[path = "model_personality_actions.rs"]
mod model_personality_actions;
#[path = "model_personality_view.rs"]
mod model_personality_view;

#[derive(Debug, Clone)]
pub(crate) enum ModelsAction {
    CacheOnly,
    ShowModels,
    ShowPersonality,
    SetPersonality(String),
}

pub(crate) fn summarize_active_personality(state: &AppState) -> String {
    model_personality_view::summarize_active_personality(state)
}

pub(crate) fn render_personality_options(cli: &Cli, state: &AppState) -> String {
    model_personality_view::render_personality_options(cli, state)
}

pub(crate) fn apply_personality_selection(
    cli: &Cli,
    state: &mut AppState,
    selector: &str,
    output: &mut Output,
) -> Result<()> {
    model_personality_actions::apply_personality_selection(cli, state, selector, output)
}

pub(crate) fn apply_models_action(
    cli: &Cli,
    state: &mut AppState,
    action: ModelsAction,
    result: &Value,
    output: &mut Output,
) -> Result<()> {
    model_personality_actions::apply_models_action(cli, state, action, result, output)
}
