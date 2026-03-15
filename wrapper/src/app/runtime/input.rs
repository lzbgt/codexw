use anyhow::Result;

use crate::Cli;
use crate::app_input_controls::handle_control_key;
use crate::app_input_editing::handle_editing_key;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::prompt_state::prompt_accepts_input;
use crate::runtime_keys::InputKey;
use crate::state::AppState;

pub(super) fn handle_input_key(
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
        &key,
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
    handle_editing_key(&key, resolved_cwd, state, editor, output, accepts_input)?;
    Ok(true)
}
