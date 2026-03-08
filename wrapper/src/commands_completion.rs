use crate::editor::LineEditor;

#[path = "commands_completion_apply.rs"]
mod commands_completion_apply;
#[path = "commands_completion_render.rs"]
mod commands_completion_render;

pub(crate) struct SlashCompletionResult {
    pub(crate) rendered_candidates: Option<String>,
}

pub(crate) fn try_complete_slash_command(
    editor: &mut LineEditor,
    buffer: &str,
    cursor_byte: usize,
) -> Option<SlashCompletionResult> {
    commands_completion_apply::try_complete_slash_command(editor, buffer, cursor_byte)
}

#[cfg(test)]
pub(crate) fn render_slash_completion_candidates(
    filter: &str,
    matches: &[&str],
    fuzzy: bool,
) -> String {
    commands_completion_render::render_slash_completion_candidates(filter, matches, fuzzy)
}

pub(crate) fn quote_if_needed(value: &str) -> String {
    commands_completion_render::quote_if_needed(value)
}
