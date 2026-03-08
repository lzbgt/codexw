use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::EditorEvent;
use crate::editor::LineEditor;
use crate::interaction::handle_tab_completion;
use crate::interaction::handle_user_input;
use crate::output::Output;
use crate::runtime::InputKey;
use crate::state::AppState;

pub(crate) fn handle_editor_key(
    key: InputKey,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
) -> Result<()> {
    match key {
        InputKey::Char(ch) => editor.insert_char(ch),
        InputKey::Backspace => editor.backspace(),
        InputKey::Delete => editor.delete(),
        InputKey::Left => editor.move_left(),
        InputKey::Right => editor.move_right(),
        InputKey::Home | InputKey::CtrlA => editor.move_home(),
        InputKey::End | InputKey::CtrlE => editor.move_end(),
        InputKey::Up => editor.history_prev(),
        InputKey::Down => editor.history_next(),
        InputKey::Tab => handle_tab_completion(editor, state, resolved_cwd, output)?,
        InputKey::CtrlU => editor.clear_to_start(),
        InputKey::CtrlW => editor.delete_prev_word(),
        InputKey::CtrlJ => editor.insert_newline(),
        InputKey::Esc | InputKey::CtrlC | InputKey::Enter => {}
    }
    Ok(())
}

pub(crate) fn handle_submit(
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    match editor.submit() {
        EditorEvent::Submit(line) => {
            output.commit_prompt(&line)?;
            handle_user_input(line, cli, resolved_cwd, state, editor, output, writer)
        }
        EditorEvent::CtrlC | EditorEvent::Noop => Ok(true),
    }
}
