use anyhow::Result;

use crate::editor::LineEditor;
use crate::output::Output;
use crate::runtime_keys::InputKey;
use crate::state::AppState;

use crate::app_input_editor::handle_editor_key;

pub(crate) fn handle_editing_key(
    key: InputKey,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    accepts_input: bool,
) -> Result<()> {
    match key {
        InputKey::Char(_)
        | InputKey::Backspace
        | InputKey::Delete
        | InputKey::Left
        | InputKey::Right
        | InputKey::Home
        | InputKey::End
        | InputKey::Up
        | InputKey::Down
        | InputKey::Tab
        | InputKey::CtrlA
        | InputKey::CtrlE
        | InputKey::CtrlU
        | InputKey::CtrlW
        | InputKey::CtrlJ => {
            if accepts_input {
                handle_editor_key(key, resolved_cwd, state, editor, output)?;
            }
        }
        _ => {}
    }
    Ok(())
}
