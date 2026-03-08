use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_submit_commands::try_handle_prefixed_submission;
use crate::dispatch_submit_turns::submit_turn_input;
use crate::editor::EditorEvent;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::prompt_completion::handle_tab_completion;
use crate::runtime_input::InputKey;
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
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Ok(true);
            }

            if let Some(result) = try_handle_prefixed_submission(
                trimmed,
                cli,
                resolved_cwd,
                state,
                editor,
                output,
                writer,
            )? {
                return Ok(result);
            }

            if !submit_turn_input(trimmed, cli, resolved_cwd, state, writer)? {
                output.line_stderr("[session] nothing to submit")?;
                return Ok(true);
            }
            Ok(true)
        }
        EditorEvent::CtrlC | EditorEvent::Noop => Ok(true),
    }
}
