use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::prompt_state::prompt_accepts_input;
use crate::runtime_input::InputKey;
use crate::state::AppState;

#[path = "app_input_controls.rs"]
mod app_input_controls;
#[path = "app_input_editing.rs"]
mod app_input_editing;
#[path = "app_input_editor.rs"]
mod app_input_editor;
#[path = "app_input_interrupt.rs"]
mod app_input_interrupt;

use app_input_controls::handle_control_key;
use app_input_editing::handle_editing_key;

pub(crate) fn handle_input_key(
    key: InputKey,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut std::process::ChildStdin,
) -> Result<bool> {
    let accepts_input = prompt_accepts_input(state);
    if let Some(continue_running) = handle_control_key(
        key,
        cli,
        resolved_cwd,
        state,
        editor,
        output,
        writer,
        accepts_input,
    )? {
        return Ok(continue_running);
    }
    handle_editing_key(key, resolved_cwd, state, editor, output, accepts_input)?;
    Ok(true)
}
