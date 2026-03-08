use anyhow::Result;

use crate::Cli;
use crate::model_catalog::ModelCatalogEntry;
use crate::model_catalog::effective_model_id;
use crate::model_personality_actions::apply_personality_selection;
use crate::model_personality_view::personality_label;
use crate::output::Output;
use crate::policy::approval_policy;
use crate::policy::thread_sandbox_mode;
use crate::render_markdown_code::available_theme_names;
use crate::render_markdown_code::current_theme_name;
use crate::render_markdown_code::set_theme;
use crate::state::AppState;
use crate::state::PendingSelection;

struct PermissionPreset {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    approval_policy: &'static str,
    thread_sandbox_mode: &'static str,
}

const PERMISSION_PRESETS: &[PermissionPreset] = &[
    PermissionPreset {
        id: "read-only",
        label: "Read Only",
        description: "Read files only; ask before edits or network access.",
        approval_policy: "on-request",
        thread_sandbox_mode: "read-only",
    },
    PermissionPreset {
        id: "auto",
        label: "Default",
        description: "Workspace writes allowed; ask before network or outside-workspace edits.",
        approval_policy: "on-request",
        thread_sandbox_mode: "workspace-write",
    },
    PermissionPreset {
        id: "full-access",
        label: "Full Access",
        description: "No approval prompts; unrestricted filesystem and network access.",
        approval_policy: "never",
        thread_sandbox_mode: "danger-full-access",
    },
];

const PERSONALITY_CHOICES: &[(&str, &str)] = &[
    ("default", "Use the backend default personality."),
    ("none", "No extra personality instructions."),
    ("friendly", "Warm, collaborative, and helpful."),
    ("pragmatic", "Concise, task-focused, and direct."),
];

pub(crate) fn pending_selection_status(selection: &PendingSelection) -> String {
    match selection {
        PendingSelection::Model => {
            "model picker | enter a number or model id | /cancel to dismiss".to_string()
        }
        PendingSelection::ReasoningEffort { model_id } => {
            format!("reasoning picker | {model_id} | enter a number or effort | /cancel to dismiss")
        }
        PendingSelection::Personality => {
            "personality picker | enter a number or label | /cancel to dismiss".to_string()
        }
        PendingSelection::Permissions => {
            "permissions picker | enter a number or preset id | /cancel to dismiss".to_string()
        }
        PendingSelection::Theme => {
            "theme picker | enter a number or theme name | /cancel to dismiss".to_string()
        }
    }
}

pub(crate) fn open_model_picker(
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    state.pending_selection = Some(PendingSelection::Model);
    Ok(output.block_stdout("Model selection", &render_model_picker(cli, state))?)
}

pub(crate) fn open_reasoning_picker(
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    model_id: &str,
) -> Result<()> {
    if find_model(state, model_id).is_none() {
        output.line_stderr(format!("[session] unknown model: {model_id}"))?;
        return Ok(());
    }
    state.pending_selection = Some(PendingSelection::ReasoningEffort {
        model_id: model_id.to_string(),
    });
    Ok(output.block_stdout(
        "Reasoning effort",
        &render_reasoning_picker(cli, state, model_id),
    )?)
}

pub(crate) fn open_personality_picker(
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    if let Some(model) = crate::model_catalog::effective_model_entry(state, cli)
        && !model.supports_personality
    {
        output.line_stderr(format!(
            "[session] model {} does not support personality overrides",
            model.display_name
        ))?;
        return Ok(());
    }
    state.pending_selection = Some(PendingSelection::Personality);
    Ok(output.block_stdout("Personality", &render_personality_picker(state))?)
}

pub(crate) fn open_permissions_picker(
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<()> {
    state.pending_selection = Some(PendingSelection::Permissions);
    Ok(output.block_stdout("Permissions", &render_permissions_picker(cli, state))?)
}

pub(crate) fn open_theme_picker(state: &mut AppState, output: &mut Output) -> Result<()> {
    state.pending_selection = Some(PendingSelection::Theme);
    Ok(output.block_stdout("Theme selection", &render_theme_picker())?)
}

pub(crate) fn handle_pending_selection(
    trimmed: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let Some(selection) = state.pending_selection.clone() else {
        return Ok(false);
    };
    if matches!(trimmed, "/cancel" | ":cancel" | "cancel") {
        state.pending_selection = None;
        output.line_stderr("[session] selection cancelled")?;
        return Ok(true);
    }

    match selection {
        PendingSelection::Model => handle_model_picker_input(trimmed, cli, state, output),
        PendingSelection::ReasoningEffort { model_id } => {
            handle_reasoning_picker_input(trimmed, state, output, &model_id)
        }
        PendingSelection::Personality => {
            handle_personality_picker_input(trimmed, cli, state, output)
        }
        PendingSelection::Permissions => {
            handle_permissions_picker_input(trimmed, cli, state, output)
        }
        PendingSelection::Theme => handle_theme_picker_input(trimmed, state, output),
    }
}

pub(crate) fn apply_permission_preset(
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

pub(crate) fn toggle_fast_mode(state: &mut AppState, output: &mut Output) -> Result<()> {
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
    Ok(())
}

pub(crate) fn apply_theme_choice(
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
    Ok(true)
}

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

fn handle_model_picker_input(
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

fn handle_reasoning_picker_input(
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

fn handle_personality_picker_input(
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

fn handle_permissions_picker_input(
    trimmed: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    apply_permission_preset(trimmed, cli, state, output)
}

fn handle_theme_picker_input(
    trimmed: &str,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    apply_theme_choice(trimmed, state, output)
}

fn render_model_picker(cli: &Cli, state: &AppState) -> String {
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

fn render_reasoning_picker(cli: &Cli, state: &AppState, model_id: &str) -> String {
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

fn render_personality_picker(state: &AppState) -> String {
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

fn render_permissions_picker(cli: &Cli, state: &AppState) -> String {
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

fn render_theme_picker() -> String {
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
    Ok(())
}

fn find_model<'a>(state: &'a AppState, model_id: &str) -> Option<&'a ModelCatalogEntry> {
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
