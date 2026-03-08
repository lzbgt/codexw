use anyhow::Result;

use crate::Cli;
use crate::editor::EditorEvent;
use crate::editor::LineEditor;
use crate::interaction::handle_tab_completion;
use crate::interaction::handle_user_input;
use crate::interaction::prompt_accepts_input;
use crate::output::Output;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_turn_interrupt;
use crate::runtime::InputKey;
use crate::state::AppState;
use crate::state::thread_id;

pub(crate) fn handle_input_key(
    key: InputKey,
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut std::process::ChildStdin,
) -> Result<bool> {
    match key {
        InputKey::Char(ch) => {
            if prompt_accepts_input(state) {
                editor.insert_char(ch);
            }
        }
        InputKey::Esc => {
            if state.turn_running {
                if let Some(turn_id) = state.active_turn_id.clone() {
                    let current_thread_id = thread_id(state)?.to_string();
                    output.line_stderr("[interrupt] interrupting active turn")?;
                    send_turn_interrupt(writer, state, current_thread_id, turn_id)?;
                } else {
                    output.line_stderr("[session] no active turn id; exiting")?;
                    return Ok(false);
                }
            } else if let Some(process_id) = state.active_exec_process_id.clone() {
                output.line_stderr("[interrupt] terminating active local command")?;
                send_command_exec_terminate(writer, state, process_id)?;
            } else if prompt_accepts_input(state) {
                editor.clear();
            }
        }
        InputKey::Backspace => {
            if prompt_accepts_input(state) {
                editor.backspace();
            }
        }
        InputKey::Delete => {
            if prompt_accepts_input(state) {
                editor.delete();
            }
        }
        InputKey::Left => {
            if prompt_accepts_input(state) {
                editor.move_left();
            }
        }
        InputKey::Right => {
            if prompt_accepts_input(state) {
                editor.move_right();
            }
        }
        InputKey::Home => {
            if prompt_accepts_input(state) {
                editor.move_home();
            }
        }
        InputKey::End => {
            if prompt_accepts_input(state) {
                editor.move_end();
            }
        }
        InputKey::Up => {
            if prompt_accepts_input(state) {
                editor.history_prev();
            }
        }
        InputKey::Down => {
            if prompt_accepts_input(state) {
                editor.history_next();
            }
        }
        InputKey::Tab => {
            if prompt_accepts_input(state) {
                handle_tab_completion(editor, state, resolved_cwd, output)?;
            }
        }
        InputKey::CtrlA => {
            if prompt_accepts_input(state) {
                editor.move_home();
            }
        }
        InputKey::CtrlE => {
            if prompt_accepts_input(state) {
                editor.move_end();
            }
        }
        InputKey::CtrlU => {
            if prompt_accepts_input(state) {
                editor.clear_to_start();
            }
        }
        InputKey::CtrlW => {
            if prompt_accepts_input(state) {
                editor.delete_prev_word();
            }
        }
        InputKey::CtrlC => {
            if state.turn_running {
                editor.clear();
                if let Some(turn_id) = state.active_turn_id.clone() {
                    let current_thread_id = thread_id(state)?.to_string();
                    output.line_stderr("[interrupt] interrupting active turn")?;
                    send_turn_interrupt(writer, state, current_thread_id, turn_id)?;
                } else {
                    output.line_stderr("[session] no active turn id; exiting")?;
                    return Ok(false);
                }
            } else if let Some(process_id) = state.active_exec_process_id.clone() {
                editor.clear();
                output.line_stderr("[interrupt] terminating active local command")?;
                send_command_exec_terminate(writer, state, process_id)?;
            } else if matches!(editor.ctrl_c(), EditorEvent::CtrlC) {
                output.line_stderr("[session] exiting on Ctrl-C")?;
                return Ok(false);
            }
        }
        InputKey::Enter => match editor.submit() {
            EditorEvent::Submit(line) => {
                output.commit_prompt(&line)?;
                if !handle_user_input(line, cli, resolved_cwd, state, editor, output, writer)? {
                    return Ok(false);
                }
            }
            EditorEvent::CtrlC | EditorEvent::Noop => {}
        },
        InputKey::CtrlJ => {
            if prompt_accepts_input(state) {
                editor.insert_newline();
            }
        }
    }
    Ok(true)
}
