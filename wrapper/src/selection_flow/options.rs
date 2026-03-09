use anyhow::Result;

use crate::Cli;
use crate::config_persistence::persist_service_tier_selection;
use crate::config_persistence::persist_theme_selection;
use crate::model_personality_actions::apply_personality_selection;
use crate::model_personality_view::personality_label;
use crate::output::Output;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::render_markdown_code::available_theme_names;
use crate::render_markdown_code::current_theme_name;
use crate::render_markdown_code::set_theme;
use crate::state::AppState;

use super::PERMISSION_PRESETS;
use super::PERSONALITY_CHOICES;
use super::PermissionPreset;

pub(super) fn apply_permission_preset(
    preset_id: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let Some(preset) = find_permission_preset(preset_id) else {
        output.line_stderr(format!("[session] unknown permissions preset: {preset_id}"))?;
        output.block_stdout("Permissions", &render_permissions_picker(cli, state))?;
        return Ok(true);
    };

    state.session_overrides.approval_policy = Some(preset.approval_policy.to_string());
    state.session_overrides.thread_sandbox_mode = Some(preset.thread_sandbox_mode.to_string());
    state.pending_selection = None;
    output.line_stderr(format!("[session] permissions updated to {}", preset.label))?;
    Ok(true)
}

pub(super) fn toggle_fast_mode(state: &mut AppState, output: &mut Output) -> Result<()> {
    let enable_fast = !matches!(
        state.session_overrides.service_tier.as_ref(),
        Some(Some(value)) if value == "fast"
    );
    state.session_overrides.service_tier = if enable_fast {
        Some(Some("fast".to_string()))
    } else {
        Some(None)
    };
    output.line_stderr(format!(
        "[session] fast mode {}",
        if enable_fast { "enabled" } else { "disabled" }
    ))?;
    if let Err(err) = persist_service_tier_selection(
        state.codex_home_override.as_deref(),
        if enable_fast { Some("fast") } else { None },
    ) {
        output.line_stderr(format!(
            "[session] failed to save service tier selection: {err:#}"
        ))?;
    }
    Ok(())
}

pub(super) fn apply_theme_choice(
    selector: &str,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let themes = available_theme_names();
    let Some(theme_name) = resolve_string_selector(selector, &themes) else {
        output.line_stderr(format!("[session] unknown theme: {selector}"))?;
        output.block_stdout("Theme selection", &render_theme_picker())?;
        return Ok(true);
    };
    set_theme(&theme_name);
    state.pending_selection = None;
    output.line_stderr(format!("[session] theme set to {theme_name}"))?;
    if let Err(err) = persist_theme_selection(state.codex_home_override.as_deref(), &theme_name) {
        output.line_stderr(format!("[session] failed to save theme selection: {err:#}"))?;
    }
    Ok(true)
}

pub(super) fn handle_personality_picker_input(
    trimmed: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let labels = PERSONALITY_CHOICES
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect::<Vec<_>>();
    let Some(selector) = resolve_string_selector(trimmed, &labels) else {
        output.line_stderr(format!("[session] unknown personality: {trimmed}"))?;
        output.block_stdout("Personality", &render_personality_picker(state))?;
        return Ok(true);
    };
    apply_personality_selection(cli, state, &selector, output)?;
    state.pending_selection = None;
    Ok(true)
}

pub(super) fn handle_permissions_picker_input(
    trimmed: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    apply_permission_preset(trimmed, cli, state, output)
}

pub(super) fn handle_theme_picker_input(
    trimmed: &str,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    apply_theme_choice(trimmed, state, output)
}

pub(super) fn render_personality_picker(state: &AppState) -> String {
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

pub(super) fn render_permissions_picker(cli: &Cli, state: &AppState) -> String {
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

pub(super) fn render_theme_picker() -> String {
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

fn find_permission_preset(selector: &str) -> Option<&'static PermissionPreset> {
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

fn resolve_string_selector(selector: &str, values: &[String]) -> Option<String> {
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
