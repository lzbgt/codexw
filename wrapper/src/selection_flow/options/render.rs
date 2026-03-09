use crate::Cli;
use crate::model_personality_view::personality_label;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::render_markdown_code::available_theme_names;
use crate::render_markdown_code::current_theme_name;
use crate::state::AppState;

use super::super::PERMISSION_PRESETS;
use super::super::PERSONALITY_CHOICES;
use super::super::PermissionPreset;

pub(in super::super) fn render_personality_picker(state: &AppState) -> String {
    PERSONALITY_CHOICES
        .iter()
        .enumerate()
        .map(|(index, (value, description))| {
            let current = match (*value, state.active_personality.as_deref()) {
                ("default", None) => " [current]".to_string(),
                (name, Some(active)) if name == active => " [current]".to_string(),
                _ => String::new(),
            };
            format!(
                "{:>2}. {}{} - {}",
                index + 1,
                personality_label(value),
                current,
                description
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(in super::super) fn render_permissions_picker(cli: &Cli, state: &AppState) -> String {
    let current_approval = approval_policy(cli, state);
    let current_sandbox = thread_sandbox_mode(cli, state);
    PERMISSION_PRESETS
        .iter()
        .enumerate()
        .map(|(index, preset)| {
            let current = if preset.approval_policy == current_approval
                && preset.thread_sandbox_mode == current_sandbox
            {
                " [current]"
            } else {
                ""
            };
            format!(
                "{:>2}. {} ({}){} - {}",
                index + 1,
                preset.label,
                preset.id,
                current,
                preset.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(in super::super) fn render_theme_picker() -> String {
    let current_theme = current_theme_name();
    available_theme_names()
        .into_iter()
        .enumerate()
        .map(|(index, theme)| {
            let current = if theme == current_theme {
                " [current]"
            } else {
                ""
            };
            format!("{:>2}. {}{}", index + 1, theme, current)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn find_permission_preset(selector: &str) -> Option<&'static PermissionPreset> {
    if let Ok(index) = selector.parse::<usize>() {
        return PERMISSION_PRESETS.get(index.saturating_sub(1));
    }
    let selector = selector.trim().to_ascii_lowercase();
    let matches = PERMISSION_PRESETS
        .iter()
        .filter(|preset| {
            preset.id.starts_with(&selector)
                || preset.label.to_ascii_lowercase().starts_with(&selector)
        })
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        Some(matches[0])
    } else {
        PERMISSION_PRESETS
            .iter()
            .find(|preset| preset.id == selector)
    }
}

pub(super) fn resolve_string_selector(selector: &str, values: &[String]) -> Option<String> {
    if let Ok(index) = selector.parse::<usize>() {
        return values.get(index.saturating_sub(1)).cloned();
    }
    let selector = selector.trim().to_ascii_lowercase();
    let matches = values
        .iter()
        .filter(|value| value.to_ascii_lowercase().starts_with(&selector))
        .cloned()
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        Some(matches[0].clone())
    } else {
        values
            .iter()
            .find(|value| value.eq_ignore_ascii_case(&selector))
            .cloned()
    }
}
