use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

use crate::commands::longest_common_prefix;
use crate::commands::try_complete_slash_command;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::state::AppState;

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

fn current_at_token(buffer: &str, cursor_byte: usize) -> Option<(usize, usize, String)> {
    let safe_cursor = clamp_to_char_boundary(buffer, cursor_byte);
    let before_cursor = &buffer[..safe_cursor];
    let after_cursor = &buffer[safe_cursor..];
    let start = before_cursor
        .char_indices()
        .rfind(|(_, ch)| ch.is_whitespace())
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or(0);
    let end_rel = after_cursor
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(after_cursor.len());
    let end = safe_cursor + end_rel;
    let token = &buffer[start..end];
    let mention = token.strip_prefix('@')?;
    if mention.is_empty() {
        return Some((start, end, String::new()));
    }
    if mention.starts_with('@') {
        return None;
    }
    if mention
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\'' | '(' | ')' | '[' | ']'))
    {
        return None;
    }
    Some((start, end, mention.to_string()))
}

fn clamp_to_char_boundary(text: &str, cursor_byte: usize) -> usize {
    if cursor_byte >= text.len() {
        return text.len();
    }
    let mut safe = cursor_byte;
    while safe > 0 && !text.is_char_boundary(safe) {
        safe -= 1;
    }
    safe
}

fn file_completions(token: &str, resolved_cwd: &str) -> Result<Vec<String>> {
    let token = token.trim();
    let (dir_part, name_prefix) = match token.rfind(['/', '\\']) {
        Some(idx) => (&token[..=idx], &token[idx + 1..]),
        None => ("", token),
    };
    let base_dir = if dir_part.is_empty() {
        PathBuf::from(resolved_cwd)
    } else {
        PathBuf::from(resolved_cwd).join(dir_part)
    };
    if !base_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut matches = std::fs::read_dir(&base_dir)
        .with_context(|| format!("read directory {}", base_dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = os_str_to_string(&name)?;
            if !name.starts_with(name_prefix) {
                return None;
            }
            let mut rendered = format!("{dir_part}{name}");
            if entry.path().is_dir() {
                rendered.push('/');
            }
            Some(rendered)
        })
        .collect::<Vec<_>>();
    matches.sort();
    Ok(matches)
}

fn os_str_to_string(value: &OsStr) -> Option<String> {
    value.to_str().map(ToOwned::to_owned)
}
