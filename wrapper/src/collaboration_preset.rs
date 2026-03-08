use serde_json::Value;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CollaborationModePreset {
    pub(crate) name: String,
    pub(crate) mode_kind: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<Option<String>>,
}

impl CollaborationModePreset {
    pub(crate) fn is_plan(&self) -> bool {
        self.mode_kind.as_deref() == Some("plan")
    }

    pub(crate) fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(mode_kind) = self.mode_kind.as_deref() {
            parts.push(format!("mode={mode_kind}"));
        }
        if let Some(model) = self.model.as_deref() {
            parts.push(format!("model={model}"));
        }
        match self.reasoning_effort.as_ref() {
            Some(Some(effort)) => parts.push(format!("effort={effort}")),
            Some(None) => parts.push("effort=default".to_string()),
            None => {}
        }

        if parts.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, parts.join(", "))
        }
    }

    pub(crate) fn turn_start_value(&self) -> Option<Value> {
        let mode = self.mode_kind.as_deref()?;
        let model = self.model.as_deref()?;
        Some(json!({
            "mode": mode,
            "settings": {
                "model": model,
                "reasoning_effort": self.reasoning_effort.clone().flatten(),
                "developer_instructions": Value::Null,
            }
        }))
    }
}

pub(crate) fn extract_collaboration_mode_presets(result: &Value) -> Vec<CollaborationModePreset> {
    result
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.to_string();
            let mode_kind = item.get("mode").and_then(Value::as_str).map(str::to_string);
            let model = item
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_string);
            let reasoning_effort = match item.get("reasoning_effort") {
                Some(Value::String(value)) => Some(Some(value.to_string())),
                Some(Value::Null) => Some(None),
                _ => None,
            };
            Some(CollaborationModePreset {
                name,
                mode_kind,
                model,
                reasoning_effort,
            })
        })
        .collect()
}

pub(crate) fn find_collaboration_mode_by_selector(
    modes: &[CollaborationModePreset],
    selector: &str,
) -> Option<CollaborationModePreset> {
    let normalized = selector.trim().to_ascii_lowercase();
    modes
        .iter()
        .find(|preset| {
            preset
                .mode_kind
                .as_deref()
                .is_some_and(|mode| mode.eq_ignore_ascii_case(&normalized))
                || preset.name.eq_ignore_ascii_case(&normalized)
        })
        .cloned()
}

pub(crate) fn current_collaboration_mode_value(
    active: Option<&CollaborationModePreset>,
) -> Option<Value> {
    active.and_then(CollaborationModePreset::turn_start_value)
}
