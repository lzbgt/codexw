use anyhow::Result;

use crate::commands_completion_render::quote_if_needed;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn emit_resume_exit_hint(
    output: &mut Output,
    state: &AppState,
    resolved_cwd: &str,
) -> Result<()> {
    if state.resume_exit_hint_emitted {
        return Ok(());
    }
    let Some(line) = build_resume_hint_line(
        &current_program_name(),
        resolved_cwd,
        state.thread_id.as_deref(),
    ) else {
        return Ok(());
    };
    output.line_stderr(line)?;
    Ok(())
}

pub(crate) fn current_program_name() -> String {
    std::env::args_os()
        .next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "codexw".to_string())
}

pub(crate) fn build_resume_hint_line(
    program: &str,
    resolved_cwd: &str,
    thread_id: Option<&str>,
) -> Option<String> {
    thread_id.map(|thread_id| {
        format!(
            "[session] resume with: {}",
            build_resume_command(program, resolved_cwd, thread_id)
        )
    })
}

pub(crate) fn build_resume_command(program: &str, resolved_cwd: &str, thread_id: &str) -> String {
    format!(
        "{} --cwd {} resume {}",
        quote_if_needed(program),
        quote_if_needed(resolved_cwd),
        quote_if_needed(thread_id)
    )
}
