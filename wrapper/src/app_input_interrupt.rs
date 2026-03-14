use std::process::ChildStdin;

use anyhow::Result;

use crate::app::build_resume_hint_line;
use crate::app::current_program_name;
use crate::editor::EditorEvent;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::PendingRequest;
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
        return interrupt_or_escalate_turn(state, output, writer, None);
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
    resolved_cwd: &str,
) -> Result<Option<bool>> {
    if state.turn_running {
        return interrupt_or_escalate_turn(state, output, writer, Some(resolved_cwd));
    } else if let Some(process_id) = state.active_exec_process_id.clone() {
        output.line_stderr("[interrupt] terminating active local command")?;
        send_command_exec_terminate(writer, state, process_id)?;
    } else if matches!(editor.ctrl_c(), EditorEvent::CtrlC) {
        output.line_stderr("[session] exiting on Ctrl-C")?;
        if let Some(line) = build_resume_hint_line(
            &current_program_name(),
            resolved_cwd,
            state.thread_id.as_deref(),
        ) {
            output.line_stderr(line)?;
            state.resume_exit_hint_emitted = true;
        }
        return Ok(Some(false));
    }
    Ok(None)
}

fn interrupt_or_escalate_turn(
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
    resolved_cwd: Option<&str>,
) -> Result<Option<bool>> {
    if turn_interrupt_already_pending(state) {
        output.line_stderr(
            "[interrupt] turn is still wedged after an interrupt request; exiting client so you can resume later",
        )?;
        if let Some(resolved_cwd) = resolved_cwd
            && let Some(line) = build_resume_hint_line(
                &current_program_name(),
                resolved_cwd,
                state.thread_id.as_deref(),
            )
        {
            output.line_stderr(line)?;
            state.resume_exit_hint_emitted = true;
        }
        return Ok(Some(false));
    }
    if let Some(turn_id) = state.active_turn_id.clone() {
        let current_thread_id = thread_id(state)?.to_string();
        output.line_stderr("[interrupt] interrupting active turn")?;
        send_turn_interrupt(writer, state, current_thread_id, turn_id)?;
    } else {
        output.line_stderr("[session] no active turn id; exiting")?;
        return Ok(Some(false));
    }
    Ok(None)
}

fn turn_interrupt_already_pending(state: &AppState) -> bool {
    state
        .pending
        .values()
        .any(|pending| matches!(pending, PendingRequest::InterruptTurn))
}
