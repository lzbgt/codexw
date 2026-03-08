use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::runtime_input::InputKey;
use crate::state::AppState;

use super::app_input_editor::handle_submit;
use super::app_input_interrupt::handle_ctrl_c;
use super::app_input_interrupt::handle_escape;

pub(crate) fn handle_control_key(
    key: InputKey,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut std::process::ChildStdin,
    accepts_input: bool,
) -> Result<Option<bool>> {
    match key {
        InputKey::Esc => handle_escape(state, editor, output, writer, accepts_input),
        InputKey::CtrlC => handle_ctrl_c(state, editor, output, writer),
        InputKey::Enter => {
            let continue_running = handle_submit(cli, resolved_cwd, state, editor, output, writer)?;
            Ok(Some(continue_running))
        }
        _ => Ok(None),
    }
}
