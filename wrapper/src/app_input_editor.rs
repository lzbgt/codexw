use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::commands_completion_apply::try_complete_slash_command;
use crate::commands_match::longest_common_prefix;
use crate::dispatch_submit_commands::try_handle_prefixed_submission;
use crate::dispatch_submit_turns::submit_turn_input;
use crate::editor::EditorEvent;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::prompt_file_completions_search::file_completions;
use crate::prompt_file_completions_token::current_at_token;
use crate::runtime_keys::InputKey;
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
        InputKey::Up => {
            if editor.is_multiline() {
                editor.move_up();
            } else {
                editor.history_prev();
            }
        }
        InputKey::Down => {
            if editor.is_multiline() {
                editor.move_down();
            } else {
                editor.history_next();
            }
        }
        InputKey::Tab => handle_tab_completion(editor, state, resolved_cwd, output)?,
        InputKey::CtrlU => editor.clear_to_start(),
        InputKey::CtrlW => editor.delete_prev_word(),
        InputKey::CtrlJ => editor.insert_newline(),
        InputKey::Esc | InputKey::CtrlC | InputKey::Enter => {}
    }
    Ok(())
}

fn handle_tab_completion(
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

pub(crate) struct FileCompletionResult {
    pub(crate) rendered_candidates: Option<String>,
}

pub(crate) fn try_complete_file_token(
    editor: &mut LineEditor,
    buffer: &str,
    cursor_byte: usize,
    resolved_cwd: &str,
) -> Result<Option<FileCompletionResult>> {
    let Some((start, end, token)) = current_at_token(buffer, cursor_byte) else {
        return Ok(None);
    };
    let completions = file_completions(&token, resolved_cwd)?;
    if completions.is_empty() {
        return Ok(None);
    }

    if completions.len() == 1 {
        editor.replace_range(start, end, &format!("{} ", completions[0]));
        return Ok(Some(FileCompletionResult {
            rendered_candidates: None,
        }));
    }

    let lcp = longest_common_prefix(&completions);
    let inserted_prefix = if lcp.len() > token.len() {
        &lcp
    } else {
        &token
    };
    editor.replace_range(start, end, &format!("@{inserted_prefix}"));
    let rendered_candidates = Some(
        completions
            .iter()
            .take(12)
            .enumerate()
            .map(|(idx, candidate)| format!("{:>2}. {}", idx + 1, candidate))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Ok(Some(FileCompletionResult {
        rendered_candidates,
    }))
}

pub(crate) fn handle_submit(
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    if state.active_exec_process_id.is_some() {
        return Ok(true);
    }

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
