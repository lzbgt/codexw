use anyhow::Result;

#[path = "selection_flow/model.rs"]
mod model;
#[path = "selection_flow/options.rs"]
mod options;

use crate::Cli;
use crate::output::Output;
use crate::state::AppState;
use crate::state::PendingSelection;

use self::model::handle_model_picker_input;
use self::model::handle_reasoning_picker_input;
use self::model::render_model_picker;
use self::model::render_reasoning_picker;
use self::options::handle_personality_picker_input;
use self::options::render_permissions_picker;
use self::options::render_personality_picker;
use self::options::render_theme_picker;

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
    if model::find_model(state, model_id).is_none() {
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
            options::handle_permissions_picker_input(trimmed, cli, state, output)
        }
        PendingSelection::Theme => options::handle_theme_picker_input(trimmed, state, output),
    }
}

pub(crate) fn apply_permission_preset(
    preset_id: &str,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    options::apply_permission_preset(preset_id, cli, state, output)
}

pub(crate) fn toggle_fast_mode(state: &mut AppState, output: &mut Output) -> Result<()> {
    options::toggle_fast_mode(state, output)
}

pub(crate) fn apply_theme_choice(
    selector: &str,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    options::apply_theme_choice(selector, state, output)
}

pub(crate) fn apply_model_choice(
    selector: &str,
    effort_override: Option<&str>,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    model::apply_model_choice(selector, effort_override, cli, state, output)
}
