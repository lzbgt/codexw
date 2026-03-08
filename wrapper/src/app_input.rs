use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::interaction::prompt_accepts_input;
use crate::output::Output;
use crate::runtime::InputKey;
use crate::state::AppState;

#[path = "app_input_editor.rs"]
mod app_input_editor;
#[path = "app_input_interrupt.rs"]
mod app_input_interrupt;

use app_input_editor::handle_editor_key;
use app_input_editor::handle_submit;
use app_input_interrupt::handle_ctrl_c;
use app_input_interrupt::handle_escape;

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
    match key {
        InputKey::Char(ch) => {
            if accepts_input {
                handle_editor_key(InputKey::Char(ch), resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::Esc => {
            if let Some(continue_running) =
                handle_escape(state, editor, output, writer, accepts_input)?
            {
                return Ok(continue_running);
            }
        }
        InputKey::Backspace => {
            if accepts_input {
                handle_editor_key(InputKey::Backspace, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::Delete => {
            if accepts_input {
                handle_editor_key(InputKey::Delete, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::Left => {
            if accepts_input {
                handle_editor_key(InputKey::Left, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::Right => {
            if accepts_input {
                handle_editor_key(InputKey::Right, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::Home => {
            if accepts_input {
                handle_editor_key(InputKey::Home, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::End => {
            if accepts_input {
                handle_editor_key(InputKey::End, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::Up => {
            if accepts_input {
                handle_editor_key(InputKey::Up, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::Down => {
            if accepts_input {
                handle_editor_key(InputKey::Down, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::Tab => {
            if accepts_input {
                handle_editor_key(InputKey::Tab, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::CtrlA => {
            if accepts_input {
                handle_editor_key(InputKey::CtrlA, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::CtrlE => {
            if accepts_input {
                handle_editor_key(InputKey::CtrlE, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::CtrlU => {
            if accepts_input {
                handle_editor_key(InputKey::CtrlU, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::CtrlW => {
            if accepts_input {
                handle_editor_key(InputKey::CtrlW, resolved_cwd, state, editor, output)?;
            }
        }
        InputKey::CtrlC => {
            if let Some(continue_running) = handle_ctrl_c(state, editor, output, writer)? {
                return Ok(continue_running);
            }
        }
        InputKey::Enter => {
            if !handle_submit(cli, resolved_cwd, state, editor, output, writer)? {
                return Ok(false);
            }
        }
        InputKey::CtrlJ => {
            if accepts_input {
                handle_editor_key(InputKey::CtrlJ, resolved_cwd, state, editor, output)?;
            }
        }
    }
    Ok(true)
}
