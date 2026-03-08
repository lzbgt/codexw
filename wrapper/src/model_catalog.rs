use serde_json::Value;

use crate::Cli;
use crate::state::AppState;
use crate::state::get_string;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelReasoningEffort {
    pub(crate) effort: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelCatalogEntry {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) supports_personality: bool,
    pub(crate) is_default: bool,
    pub(crate) default_reasoning_effort: Option<String>,
    pub(crate) supported_reasoning_efforts: Vec<ModelReasoningEffort>,
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
                .or_else(|| get_string(model, &["display_name"]))
                .unwrap_or(id.as_str())
                .to_string();
            let description = get_string(model, &["description"])
                .unwrap_or("")
                .to_string();
            let supports_personality = model
                .get("supportsPersonality")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let is_default = model
                .get("isDefault")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let default_reasoning_effort =
                get_string(model, &["defaultReasoningLevel", "default_reasoning_level"])
                    .map(ToString::to_string);
            let supported_reasoning_efforts = model
                .get("supportedReasoningLevels")
                .or_else(|| model.get("supported_reasoning_levels"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|entry| {
                    let effort = get_string(entry, &["effort"])?;
                    Some(ModelReasoningEffort {
                        effort: effort.to_string(),
                        description: get_string(entry, &["description"])
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect::<Vec<_>>();
            Some(ModelCatalogEntry {
                id,
                display_name,
                description,
                supports_personality,
                is_default,
                default_reasoning_effort,
                supported_reasoning_efforts,
            })
        })
        .collect()
}

pub(crate) fn effective_model_id<'a>(state: &'a AppState, cli: &'a Cli) -> Option<&'a str> {
    if let Some(model) = state
        .session_overrides
        .model
        .as_ref()
        .and_then(|value| value.as_deref())
    {
        return Some(model);
    }
    if let Some(model) = state
        .active_collaboration_mode
        .as_ref()
        .and_then(|preset| preset.model.as_deref())
    {
        return Some(model);
    }
    if let Some(model) = cli.model.as_deref() {
        return Some(model);
    }
    state
        .models
        .iter()
        .find(|entry| entry.is_default)
        .map(|entry| entry.id.as_str())
}

pub(crate) fn effective_model_entry<'a>(
    state: &'a AppState,
    cli: &'a Cli,
) -> Option<&'a ModelCatalogEntry> {
    let model_id = effective_model_id(state, cli)?;
    state.models.iter().find(|entry| entry.id == model_id)
}
