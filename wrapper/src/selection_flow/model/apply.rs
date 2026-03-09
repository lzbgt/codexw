use anyhow::Result;

use crate::Cli;
use crate::config_persistence::persist_model_selection;
use crate::model_catalog::ModelCatalogEntry;
use crate::output::Output;
use crate::state::AppState;

use super::render::find_model_by_selector;
use super::render::find_reasoning_effort;
use super::render::render_model_picker;
use super::render::render_reasoning_picker;
use super::render::render_reasoning_picker_for_model;

pub(crate) fn apply_model_choice(
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

pub(crate) fn handle_model_picker_input(
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

    super::super::open_reasoning_picker(cli, state, output, &model.id)?;
    Ok(true)
}

pub(crate) fn handle_reasoning_picker_input(
    trimmed: &str,
    state: &mut AppState,
    output: &mut Output,
    model_id: &str,
) -> Result<bool> {
    let Some(model) = super::render::find_model(state, model_id).cloned() else {
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
