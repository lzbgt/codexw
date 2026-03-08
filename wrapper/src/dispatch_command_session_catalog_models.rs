use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::model_session::ModelsAction;
use crate::model_session::apply_personality_selection;
use crate::model_session::render_personality_options;
use crate::output::Output;
use crate::requests::send_load_models;
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
        "models" | "model" => {
            output.line_stderr("[session] loading models")?;
            send_load_models(writer, state, ModelsAction::ShowModels)?;
            true
        }
        "personality" => {
            if state.turn_running {
                output
                    .line_stderr("[session] cannot change personality while a turn is running")?;
            } else if args.is_empty() {
                if state.models.is_empty() {
                    output.line_stderr("[session] loading models for personality options")?;
                    send_load_models(writer, state, ModelsAction::ShowPersonality)?;
                } else {
                    output.block_stdout("Personality", &render_personality_options(cli, state))?;
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
