use anyhow::Result;

use crate::Cli;
use crate::config_persistence::persist_service_tier_selection;
use crate::config_persistence::persist_theme_selection;
use crate::model_personality_actions::apply_personality_selection;
use crate::output::Output;
use crate::state::AppState;

use super::super::PERMISSION_PRESETS;
use super::super::PERSONALITY_CHOICES;
use super::render::find_permission_preset;
use super::render::render_permissions_picker;
use super::render::render_personality_picker;
use super::render::render_theme_picker;
use super::render::resolve_string_selector;

pub(in super::super) fn apply_permission_preset(
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

pub(in super::super) fn toggle_fast_mode(state: &mut AppState, output: &mut Output) -> Result<()> {
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

pub(in super::super) fn apply_theme_choice(
    selector: &str,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let themes = crate::render_markdown_code::available_theme_names();
    let Some(theme_name) = resolve_string_selector(selector, &themes) else {
        output.line_stderr(format!("[session] unknown theme: {selector}"))?;
        output.block_stdout("Theme selection", &render_theme_picker())?;
        return Ok(true);
    };
    crate::render_markdown_code::set_theme(&theme_name);
    state.pending_selection = None;
    output.line_stderr(format!("[session] theme set to {theme_name}"))?;
    if let Err(err) = persist_theme_selection(state.codex_home_override.as_deref(), &theme_name) {
        output.line_stderr(format!("[session] failed to save theme selection: {err:#}"))?;
    }
    Ok(true)
}

pub(in super::super) fn handle_personality_picker_input(
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

pub(in super::super) fn handle_permissions_picker_input(
    trimmed: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    apply_permission_preset(trimmed, cli, state, output)
}

pub(in super::super) fn handle_theme_picker_input(
    trimmed: &str,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    apply_theme_choice(trimmed, state, output)
}

#[allow(dead_code)]
const _: usize = PERMISSION_PRESETS.len();
