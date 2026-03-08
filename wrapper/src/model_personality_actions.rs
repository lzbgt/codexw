use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::catalog_backend_views::render_models_list;
use crate::config_persistence::persist_personality_selection;
use crate::model_catalog::effective_model_entry;
use crate::model_catalog::extract_models;
use crate::output::Output;
use crate::selection_flow::apply_model_choice;
use crate::selection_flow::open_model_picker;
use crate::selection_flow::open_personality_picker;
use crate::state::AppState;

use crate::model_personality_view::personality_label;
use crate::model_personality_view::render_personality_options;

#[derive(Debug, Clone)]
pub(crate) enum ModelsAction {
    CacheOnly,
    ShowModels,
    OpenModelPicker,
    OpenPersonalityPicker,
    SetModel {
        selector: String,
        effort: Option<String>,
    },
    SetPersonality(String),
}

pub(crate) fn apply_personality_selection(
    cli: &Cli,
    state: &mut AppState,
    selector: &str,
    output: &mut Output,
) -> Result<()> {
    let normalized = selector.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "default" | "clear") {
        state.active_personality = None;
        state.session_overrides.personality = Some(None);
        output.line_stderr("[session] personality cleared; using backend default")?;
        if let Err(err) = persist_personality_selection(state.codex_home_override.as_deref(), None)
        {
            output.line_stderr(format!(
                "[session] failed to save personality selection: {err:#}"
            ))?;
        }
        return Ok(());
    }
    if !matches!(normalized.as_str(), "none" | "friendly" | "pragmatic") {
        output.line_stderr(format!("[session] unknown personality: {selector}"))?;
        output.block_stdout("Personality", &render_personality_options(cli, state))?;
        return Ok(());
    }
    if let Some(model) = effective_model_entry(state, cli)
        && !model.supports_personality
    {
        output.line_stderr(format!(
            "[session] model {} does not support personality overrides",
            model.display_name
        ))?;
        return Ok(());
    }
    state.active_personality = Some(normalized.clone());
    state.session_overrides.personality = Some(Some(normalized.clone()));
    output.line_stderr(format!(
        "[session] personality set to {}",
        personality_label(&normalized)
    ))?;
    if let Err(err) =
        persist_personality_selection(state.codex_home_override.as_deref(), Some(&normalized))
    {
        output.line_stderr(format!(
            "[session] failed to save personality selection: {err:#}"
        ))?;
    }
    Ok(())
}

pub(crate) fn apply_models_action(
    cli: &Cli,
    state: &mut AppState,
    action: ModelsAction,
    result: &Value,
    output: &mut Output,
) -> Result<()> {
    state.models = extract_models(result);
    match action {
        ModelsAction::CacheOnly => {}
        ModelsAction::ShowModels => {
            output.block_stdout("Models", &render_models_list(result))?;
        }
        ModelsAction::OpenModelPicker => {
            open_model_picker(cli, state, output)?;
        }
        ModelsAction::OpenPersonalityPicker => {
            open_personality_picker(cli, state, output)?;
        }
        ModelsAction::SetModel { selector, effort } => {
            apply_model_choice(&selector, effort.as_deref(), cli, state, output)?;
        }
        ModelsAction::SetPersonality(selector) => {
            apply_personality_selection(cli, state, &selector, output)?;
        }
    }
    Ok(())
}
