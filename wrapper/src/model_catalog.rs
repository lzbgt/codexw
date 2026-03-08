use serde_json::Value;

use crate::Cli;
use crate::state::AppState;
use crate::state::get_string;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelCatalogEntry {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) supports_personality: bool,
    pub(crate) is_default: bool,
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
