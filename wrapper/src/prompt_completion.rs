#[path = "prompt_file_completions.rs"]
mod prompt_file_completions;

use anyhow::Result;

use crate::commands_completion_apply::try_complete_slash_command;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

pub(crate) use prompt_file_completions::try_complete_file_token;

pub(crate) fn handle_tab_completion(
    editor: &mut LineEditor,
    state: &AppState,
    resolved_cwd: &str,
    output: &mut Output,
) -> Result<()> {
    let buffer = editor.buffer().to_string();
    let cursor_byte = editor.cursor_byte_index();

    if let Some(result) = try_complete_slash_command(editor, &buffer, cursor_byte) {
        if let Some(rendered) = result.rendered_candidates {
            output.block_stdout("Command completions", &rendered)?;
        }
        return Ok(());
    }

    if let Some(result) = try_complete_file_token(editor, &buffer, cursor_byte, resolved_cwd)? {
        if let Some(rendered) = result.rendered_candidates {
            output.block_stdout("File completions", &rendered)?;
        }
        return Ok(());
    }

    if !state.turn_running && !buffer.trim_start().starts_with('!') {
        output.line_stderr("[tab] no completion available")?;
    }
    Ok(())
}
