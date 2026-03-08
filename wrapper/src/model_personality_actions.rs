use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::catalog_views::render_models_list;
use crate::model_catalog::effective_model_entry;
use crate::model_catalog::extract_models;
use crate::output::Output;
use crate::state::AppState;

use super::ModelsAction;
use super::model_personality_view::personality_label;
use super::model_personality_view::render_personality_options;

pub(crate) fn apply_personality_selection(
    cli: &Cli,
    state: &mut AppState,
    selector: &str,
    output: &mut Output,
) -> Result<()> {
    let normalized = selector.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "default" | "clear") {
        state.active_personality = None;
        output.line_stderr("[session] personality cleared; using backend default")?;
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
    output.line_stderr(format!(
        "[session] personality set to {}",
        personality_label(&normalized)
    ))?;
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
        ModelsAction::ShowPersonality => {
            output.block_stdout("Personality", &render_personality_options(cli, state))?;
        }
        ModelsAction::SetPersonality(selector) => {
            apply_personality_selection(cli, state, &selector, output)?;
        }
    }
    Ok(())
}
