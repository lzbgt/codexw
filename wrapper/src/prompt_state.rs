use anyhow::Context;
use anyhow::Result;

use crate::editor::LineEditor;
use crate::output::Output;
use crate::session_prompt_status::render_prompt_status;
use crate::state::AppState;

pub(crate) fn update_prompt(
    output: &mut Output,
    state: &AppState,
    editor: &LineEditor,
) -> Result<()> {
    let prompt = prompt_is_visible(state).then(String::new);
    let status = prompt_is_visible(state).then(|| render_prompt_status(state));
    output.set_prompt(prompt);
    output.set_status(status);
    output
        .show_prompt(editor.buffer(), editor.cursor_chars())
        .context("show prompt")
}

pub(crate) fn prompt_is_visible(state: &AppState) -> bool {
    state.thread_id.is_some() && !state.pending_thread_switch
}

pub(crate) fn prompt_accepts_input(state: &AppState) -> bool {
    prompt_is_visible(state) && state.active_exec_process_id.is_none()
}
