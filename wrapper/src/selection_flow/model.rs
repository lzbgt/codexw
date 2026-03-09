use anyhow::Result;

use crate::Cli;
use crate::config_persistence::persist_model_selection;
use crate::model_catalog::ModelCatalogEntry;
use crate::model_catalog::effective_model_id;
use crate::output::Output;
use crate::state::AppState;

use super::open_reasoning_picker;

pub(super) fn apply_model_choice(
    selector: &str,
    effort_override: Option<&str>,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    if matches!(selector.trim(), "default" | "auto" | "clear") {
        state.session_overrides.model = Some(None);
        state.session_overrides.reasoning_effort = Some(None);
        state.pending_selection = None;
        output.line_stderr("[session] model reset to backend default")?;
        if let Err(err) = persist_model_selection(state.codex_home_override.as_deref(), None, None)
        {
            output.line_stderr(format!("[session] failed to save model selection: {err:#}"))?;
        }
        return Ok(true);
    }

    let Some(model) = find_model_by_selector(state, selector).cloned() else {
        output.line_stderr(format!("[session] unknown model: {selector}"))?;
        output.block_stdout("Model selection", &render_model_picker(cli, state))?;
        return Ok(true);
    };

    let selected_effort = if let Some(effort) = effort_override {
        let Some(effort_value) = find_reasoning_effort(&model, effort) else {
            output.line_stderr(format!("[session] unknown reasoning effort: {effort}"))?;
            output.block_stdout(
                "Reasoning effort",
                &render_reasoning_picker(cli, state, &model.id),
            )?;
            return Ok(true);
        };
        Some(effort_value.to_string())
    } else {
        model.default_reasoning_effort.clone()
    };

    apply_model_and_effort(state, output, &model, selected_effort.as_deref())?;
    Ok(true)
}

pub(super) fn handle_model_picker_input(
    trimmed: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    if matches!(trimmed, "default" | "auto" | "clear") {
        return apply_model_choice(trimmed, None, cli, state, output);
    }

    let Some(model) = find_model_by_selector(state, trimmed).cloned() else {
        output.line_stderr(format!("[session] unknown model: {trimmed}"))?;
        output.block_stdout("Model selection", &render_model_picker(cli, state))?;
        return Ok(true);
    };

    if model.supported_reasoning_efforts.len() <= 1 {
        let effort = model.default_reasoning_effort.as_deref();
        apply_model_and_effort(state, output, &model, effort)?;
        return Ok(true);
    }

    open_reasoning_picker(cli, state, output, &model.id)?;
    Ok(true)
}

pub(super) fn handle_reasoning_picker_input(
    trimmed: &str,
    state: &mut AppState,
    output: &mut Output,
    model_id: &str,
) -> Result<bool> {
    let Some(model) = find_model(state, model_id).cloned() else {
        state.pending_selection = None;
        output.line_stderr("[session] model catalog changed; reopen /model")?;
        return Ok(true);
    };
    let Some(effort) = find_reasoning_effort(&model, trimmed) else {
        output.line_stderr(format!("[session] unknown reasoning effort: {trimmed}"))?;
        output.block_stdout(
            "Reasoning effort",
            &render_reasoning_picker_for_model(state, &model),
        )?;
        return Ok(true);
    };
    apply_model_and_effort(state, output, &model, Some(effort))?;
    Ok(true)
}

pub(super) fn render_model_picker(cli: &Cli, state: &AppState) -> String {
    if state.models.is_empty() {
        return "No models returned by app-server.".to_string();
    }
    let current_model = effective_model_id(state, cli);
    let current_effort = state
        .session_overrides
        .reasoning_effort
        .as_ref()
        .and_then(|value| value.as_deref());
    state
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| {
            let mut markers = Vec::new();
            if current_model == Some(model.id.as_str()) {
                markers.push("current".to_string());
            }
            if model.is_default {
                markers.push("default".to_string());
            }
            if model.supports_personality {
                markers.push("supports personality".to_string());
            }
            let effort = if current_model == Some(model.id.as_str()) {
                current_effort.or(model.default_reasoning_effort.as_deref())
            } else {
                model.default_reasoning_effort.as_deref()
            };
            let marker_suffix = if markers.is_empty() {
                String::new()
            } else {
                format!(" [{}]", markers.join(", "))
            };
            let effort_suffix = effort.map_or_else(String::new, |value| format!(" effort={value}"));
            let description_suffix = if model.description.is_empty() {
                String::new()
            } else {
                format!(" - {}", model.description)
            };
            format!(
                "{:>2}. {} ({}){}{}{}",
                index + 1,
                model.display_name,
                model.id,
                marker_suffix,
                effort_suffix,
                description_suffix
            )
        })
        .chain(std::iter::once(
            "Enter a number or model id. Use `default` to clear the override.".to_string(),
        ))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn render_reasoning_picker(cli: &Cli, state: &AppState, model_id: &str) -> String {
    find_model(state, model_id)
        .map(|model| render_reasoning_picker_for_model_with_cli(cli, state, model))
        .unwrap_or_else(|| "Model is no longer available.".to_string())
}

fn render_reasoning_picker_for_model(state: &AppState, model: &ModelCatalogEntry) -> String {
    let selected_model = state
        .session_overrides
        .model
        .as_ref()
        .and_then(|value| value.as_deref());
    let current_effort = state
        .session_overrides
        .reasoning_effort
        .as_ref()
        .and_then(|value| value.as_deref());
    render_reasoning_lines(
        model,
        selected_model == Some(model.id.as_str()),
        current_effort,
    )
}

fn render_reasoning_picker_for_model_with_cli(
    cli: &Cli,
    state: &AppState,
    model: &ModelCatalogEntry,
) -> String {
    let current_model = effective_model_id(state, cli);
    let current_effort = state
        .session_overrides
        .reasoning_effort
        .as_ref()
        .and_then(|value| value.as_deref());
    render_reasoning_lines(
        model,
        current_model == Some(model.id.as_str()),
        current_effort,
    )
}

fn render_reasoning_lines(
    model: &ModelCatalogEntry,
    is_current_model: bool,
    current_effort: Option<&str>,
) -> String {
    model
        .supported_reasoning_efforts
        .iter()
        .enumerate()
        .map(|(index, effort)| {
            let mut markers = Vec::new();
            if model.default_reasoning_effort.as_deref() == Some(effort.effort.as_str()) {
                markers.push("default".to_string());
            }
            if is_current_model && current_effort == Some(effort.effort.as_str()) {
                markers.push("current".to_string());
            }
            let marker_suffix = if markers.is_empty() {
                String::new()
            } else {
                format!(" [{}]", markers.join(", "))
            };
            let description_suffix = if effort.description.is_empty() {
                String::new()
            } else {
                format!(" - {}", effort.description)
            };
            format!(
                "{:>2}. {}{}{}",
                index + 1,
                effort.effort,
                marker_suffix,
                description_suffix
            )
        })
        .chain(std::iter::once(
            "Enter a number or effort name such as `medium` or `high`.".to_string(),
        ))
        .collect::<Vec<_>>()
        .join("\n")
}

fn apply_model_and_effort(
    state: &mut AppState,
    output: &mut Output,
    model: &ModelCatalogEntry,
    effort: Option<&str>,
) -> Result<()> {
    state.session_overrides.model = Some(Some(model.id.clone()));
    state.session_overrides.reasoning_effort = Some(effort.map(str::to_string));
    state.pending_selection = None;
    if let Some(effort) = effort {
        output.line_stderr(format!(
            "[session] model set to {} ({}) effort={effort}",
            model.display_name, model.id
        ))?;
    } else {
        output.line_stderr(format!(
            "[session] model set to {} ({})",
            model.display_name, model.id
        ))?;
    }
    if let Err(err) = persist_model_selection(
        state.codex_home_override.as_deref(),
        Some(&model.id),
        effort,
    ) {
        output.line_stderr(format!("[session] failed to save model selection: {err:#}"))?;
    }
    Ok(())
}

pub(super) fn find_model<'a>(state: &'a AppState, model_id: &str) -> Option<&'a ModelCatalogEntry> {
    state.models.iter().find(|model| model.id == model_id)
}

fn find_model_by_selector<'a>(
    state: &'a AppState,
    selector: &str,
) -> Option<&'a ModelCatalogEntry> {
    if let Ok(index) = selector.parse::<usize>() {
        return state.models.get(index.saturating_sub(1));
    }
    let selector = selector.trim().to_ascii_lowercase();
    state
        .models
        .iter()
        .find(|model| model.id.eq_ignore_ascii_case(&selector))
        .or_else(|| {
            state
                .models
                .iter()
                .find(|model| model.display_name.eq_ignore_ascii_case(&selector))
        })
        .or_else(|| {
            let matches = state
                .models
                .iter()
                .filter(|model| {
                    model.id.to_ascii_lowercase().starts_with(&selector)
                        || model
                            .display_name
                            .to_ascii_lowercase()
                            .starts_with(&selector)
                })
                .collect::<Vec<_>>();
            (matches.len() == 1).then_some(matches[0])
        })
}

fn find_reasoning_effort<'a>(model: &'a ModelCatalogEntry, selector: &str) -> Option<&'a str> {
    if let Ok(index) = selector.parse::<usize>() {
        return model
            .supported_reasoning_efforts
            .get(index.saturating_sub(1))
            .map(|effort| effort.effort.as_str());
    }
    let selector = selector.trim().to_ascii_lowercase();
    let matches = model
        .supported_reasoning_efforts
        .iter()
        .filter(|effort| effort.effort.starts_with(&selector))
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        Some(matches[0].effort.as_str())
    } else {
        model
            .supported_reasoning_efforts
            .iter()
            .find(|effort| effort.effort == selector)
            .map(|effort| effort.effort.as_str())
    }
}
