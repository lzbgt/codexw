use anyhow::Result;
use serde_json::Value;
use serde_json::json;

use crate::output::Output;
use crate::state::AppState;

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

#[derive(Debug, Clone)]
pub(crate) enum CollaborationModeAction {
    CacheOnly,
    ShowList,
    TogglePlan,
    SetMode(String),
}

pub(crate) fn current_collaboration_mode_value(state: &AppState) -> Option<Value> {
    state
        .active_collaboration_mode
        .as_ref()
        .and_then(CollaborationModePreset::turn_start_value)
}

pub(crate) fn summarize_active_collaboration_mode(state: &AppState) -> String {
    state
        .active_collaboration_mode
        .as_ref()
        .map(CollaborationModePreset::summary)
        .unwrap_or_else(|| "default".to_string())
}

pub(crate) fn current_collaboration_mode_label(state: &AppState) -> Option<String> {
    let preset = state.active_collaboration_mode.as_ref()?;
    if preset.is_plan() {
        Some("plan mode".to_string())
    } else {
        Some(format!("collab {}", preset.name))
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

fn find_collaboration_mode_by_selector(
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

pub(crate) fn render_collaboration_modes(state: &AppState) -> String {
    let current = summarize_active_collaboration_mode(state);
    if state.collaboration_modes.is_empty() {
        return format!(
            "current         {current}\nno collaboration mode presets available from app-server"
        );
    }

    let mut lines = vec![
        format!("current         {current}"),
        "available presets".to_string(),
    ];
    for (index, preset) in state.collaboration_modes.iter().enumerate() {
        lines.push(format!(" {:>2}. {}", index + 1, preset.summary()));
    }
    lines.push("Use /collab <name|mode> or /plan to switch.".to_string());
    lines.join("\n")
}

pub(crate) fn apply_collaboration_mode_action(
    state: &mut AppState,
    action: CollaborationModeAction,
    output: &mut Output,
) -> Result<()> {
    match action {
        CollaborationModeAction::CacheOnly => {}
        CollaborationModeAction::ShowList => {
            output.block_stdout("Collaboration modes", &render_collaboration_modes(state))?;
        }
        CollaborationModeAction::TogglePlan => {
            if state
                .active_collaboration_mode
                .as_ref()
                .is_some_and(CollaborationModePreset::is_plan)
            {
                state.active_collaboration_mode = None;
                output.line_stderr("[session] collaboration mode cleared; using default mode")?;
            } else if let Some(plan) = state
                .collaboration_modes
                .iter()
                .find(|preset| preset.is_plan())
                .cloned()
            {
                let summary = plan.summary();
                state.active_collaboration_mode = Some(plan);
                output.line_stderr(format!("[session] switched to {summary}"))?;
            } else {
                output.line_stderr(
                    "[session] no plan collaboration preset is available from app-server",
                )?;
            }
        }
        CollaborationModeAction::SetMode(selector) => {
            let normalized = selector.trim().to_ascii_lowercase();
            if matches!(normalized.as_str(), "default" | "off" | "none" | "clear") {
                state.active_collaboration_mode = None;
                output.line_stderr("[session] collaboration mode cleared; using default mode")?;
            } else if let Some(preset) =
                find_collaboration_mode_by_selector(&state.collaboration_modes, &normalized)
            {
                let summary = preset.summary();
                state.active_collaboration_mode = Some(preset);
                output.line_stderr(format!("[session] switched to {summary}"))?;
            } else if state.collaboration_modes.is_empty() {
                output.line_stderr(
                    "[session] no collaboration mode presets are available from app-server",
                )?;
            } else {
                output.line_stderr(format!("[session] unknown collaboration mode: {selector}"))?;
                output.block_stdout("Collaboration modes", &render_collaboration_modes(state))?;
            }
        }
    }

    Ok(())
}
