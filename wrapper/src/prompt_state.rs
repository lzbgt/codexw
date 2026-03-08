use anyhow::Context;
use anyhow::Result;

use crate::editor::LineEditor;
use crate::output::Output;
use crate::session_prompt_status_active;
use crate::session_prompt_status_ready;
use crate::state::AppState;

pub(crate) fn update_prompt(
    output: &mut Output,
    state: &AppState,
    editor: &LineEditor,
) -> Result<()> {
    let prompt = prompt_is_visible(state).then(|| {
        if state.startup_resume_picker && state.thread_id.is_none() {
            "resume".to_string()
        } else {
            String::new()
        }
    });
    let status = prompt_is_visible(state).then(|| render_prompt_status(state));
    output.set_prompt(prompt);
    output.set_status(status);
    output
        .show_prompt(editor.buffer(), editor.cursor_chars())
        .context("show prompt")
}

pub(crate) fn render_prompt_status(state: &AppState) -> String {
    if state.startup_resume_picker && state.thread_id.is_none() {
        "resume picker | enter a number or thread id | /new for a fresh thread".to_string()
    } else if state.active_exec_process_id.is_some() {
        session_prompt_status_active::render_exec_status(state)
    } else if state.turn_running {
        session_prompt_status_active::render_turn_status(state)
    } else if state.realtime_active {
        session_prompt_status_active::render_realtime_status(state)
    } else {
        session_prompt_status_ready::render_ready_status(state)
    }
}

pub(crate) fn prompt_is_visible(state: &AppState) -> bool {
    (state.thread_id.is_some() || state.startup_resume_picker) && !state.pending_thread_switch
}

pub(crate) fn prompt_accepts_input(state: &AppState) -> bool {
    prompt_is_visible(state) && state.active_exec_process_id.is_none()
}
