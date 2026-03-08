use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::model_personality_actions::ModelsAction;
use crate::model_personality_actions::apply_personality_selection;
use crate::output::Output;
use crate::requests::send_load_models;
use crate::selection_flow::apply_model_choice;
use crate::selection_flow::open_model_picker;
use crate::selection_flow::open_personality_picker;
use crate::state::AppState;

pub(crate) fn try_handle_session_catalog_model_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "models" => {
            output.line_stderr("[session] loading models")?;
            send_load_models(writer, state, ModelsAction::ShowModels)?;
            true
        }
        "model" => {
            if state.turn_running {
                output.line_stderr("[session] cannot change model while a turn is running")?;
            } else if args.is_empty() {
                if state.models.is_empty() {
                    output.line_stderr("[session] loading models for selection")?;
                    send_load_models(writer, state, ModelsAction::OpenModelPicker)?;
                } else {
                    open_model_picker(cli, state, output)?;
                }
            } else {
                let selector = args.first().copied().unwrap_or_default().to_string();
                let effort = args.get(1).map(|value| (*value).to_string());
                if state.models.is_empty() {
                    output.line_stderr("[session] loading models for selection")?;
                    send_load_models(writer, state, ModelsAction::SetModel { selector, effort })?;
                } else {
                    apply_model_choice(&selector, effort.as_deref(), cli, state, output)?;
                }
            }
            true
        }
        "personality" => {
            if state.turn_running {
                output
                    .line_stderr("[session] cannot change personality while a turn is running")?;
            } else if args.is_empty() {
                if state.models.is_empty() {
                    output.line_stderr("[session] loading models for personality selection")?;
                    send_load_models(writer, state, ModelsAction::OpenPersonalityPicker)?;
                } else {
                    open_personality_picker(cli, state, output)?;
                }
            } else {
                let selector = args.join(" ");
                if state.models.is_empty() {
                    output.line_stderr("[session] loading models for personality selection")?;
                    send_load_models(writer, state, ModelsAction::SetPersonality(selector))?;
                } else {
                    apply_personality_selection(cli, state, &selector, output)?;
                }
            }
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}
