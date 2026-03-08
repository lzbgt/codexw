use anyhow::Result;

use crate::commands_match::longest_common_prefix;
use crate::editor::LineEditor;
use crate::prompt_file_completions_search::file_completions;
use crate::prompt_file_completions_token::current_at_token;

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
