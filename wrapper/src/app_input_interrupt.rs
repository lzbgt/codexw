use std::process::ChildStdin;

use anyhow::Result;

use crate::editor::EditorEvent;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::send_command_exec_terminate;
use crate::requests::send_turn_interrupt;
use crate::state::AppState;
use crate::state::thread_id;

pub(crate) fn handle_escape(
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
    prompt_accepts_input: bool,
) -> Result<Option<bool>> {
    if state.turn_running {
        if let Some(turn_id) = state.active_turn_id.clone() {
            let current_thread_id = thread_id(state)?.to_string();
            output.line_stderr("[interrupt] interrupting active turn")?;
            send_turn_interrupt(writer, state, current_thread_id, turn_id)?;
        } else {
            output.line_stderr("[session] no active turn id; exiting")?;
            return Ok(Some(false));
        }
    } else if let Some(process_id) = state.active_exec_process_id.clone() {
        output.line_stderr("[interrupt] terminating active local command")?;
        send_command_exec_terminate(writer, state, process_id)?;
    } else if prompt_accepts_input {
        editor.clear();
    }
    Ok(None)
}

pub(crate) fn handle_ctrl_c(
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    if state.turn_running {
        editor.clear();
        if let Some(turn_id) = state.active_turn_id.clone() {
            let current_thread_id = thread_id(state)?.to_string();
            output.line_stderr("[interrupt] interrupting active turn")?;
            send_turn_interrupt(writer, state, current_thread_id, turn_id)?;
        } else {
            output.line_stderr("[session] no active turn id; exiting")?;
            return Ok(Some(false));
        }
    } else if let Some(process_id) = state.active_exec_process_id.clone() {
        editor.clear();
        output.line_stderr("[interrupt] terminating active local command")?;
        send_command_exec_terminate(writer, state, process_id)?;
    } else if matches!(editor.ctrl_c(), EditorEvent::CtrlC) {
        output.line_stderr("[session] exiting on Ctrl-C")?;
        return Ok(Some(false));
    }
    Ok(None)
}
