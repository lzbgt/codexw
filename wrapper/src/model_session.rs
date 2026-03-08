use anyhow::Result;
use serde_json::Value;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;
use crate::state::get_string;
use crate::views::render_models_list;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelCatalogEntry {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) supports_personality: bool,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum ModelsAction {
    CacheOnly,
    ShowModels,
    ShowPersonality,
    SetPersonality(String),
}

fn personality_label(personality: &str) -> &str {
    match personality {
        "none" => "None",
        "friendly" => "Friendly",
        "pragmatic" => "Pragmatic",
        _ => personality,
    }
}

pub(crate) fn summarize_active_personality(state: &AppState) -> String {
    state
        .active_personality
        .as_deref()
        .map(|value| personality_label(value).to_string())
        .unwrap_or_else(|| "default".to_string())
}

pub(crate) fn extract_models(result: &Value) -> Vec<ModelCatalogEntry> {
    result
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let id = get_string(model, &["id"])
                .or_else(|| get_string(model, &["model"]))?
                .to_string();
            let display_name = get_string(model, &["displayName"])
                .unwrap_or(id.as_str())
                .to_string();
            let supports_personality = model
                .get("supportsPersonality")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let is_default = model
                .get("isDefault")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Some(ModelCatalogEntry {
                id,
                display_name,
                supports_personality,
                is_default,
            })
        })
        .collect()
}

pub(crate) fn effective_model_entry<'a>(
    state: &'a AppState,
    cli: &Cli,
) -> Option<&'a ModelCatalogEntry> {
    if let Some(model) = state
        .active_collaboration_mode
        .as_ref()
        .and_then(|preset| preset.model.as_deref())
    {
        return state.models.iter().find(|entry| entry.id == model);
    }
    if let Some(model) = cli.model.as_deref() {
        return state.models.iter().find(|entry| entry.id == model);
    }
    state.models.iter().find(|entry| entry.is_default)
}

pub(crate) fn render_personality_options(cli: &Cli, state: &AppState) -> String {
    let support_line = match effective_model_entry(state, cli) {
        Some(model) if model.supports_personality => format!(
            "current model     {} [supports personality]",
            model.display_name
        ),
        Some(model) => format!(
            "current model     {} [personality unsupported]",
            model.display_name
        ),
        None => "current model     unknown".to_string(),
    };
    [
        format!("current          {}", summarize_active_personality(state)),
        support_line,
        "available choices".to_string(),
        "  - friendly  Warm, collaborative, and helpful.".to_string(),
        "  - pragmatic Concise, task-focused, and direct.".to_string(),
        "  - none      No extra personality instructions.".to_string(),
        "Use /personality <friendly|pragmatic|none|default> to change it.".to_string(),
    ]
    .join("\n")
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
