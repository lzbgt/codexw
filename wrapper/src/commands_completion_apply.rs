use crate::commands_catalog::builtin_command_names;
use crate::commands_match::fuzzy_match_score;
use crate::commands_match::longest_common_prefix;
use crate::commands_match::slash_command_at_cursor;
use crate::editor::LineEditor;

use crate::commands_completion_render::render_slash_completion_candidates;

pub(crate) struct SlashCompletionResult {
    pub(crate) rendered_candidates: Option<String>,
}

pub(crate) fn try_complete_slash_command(
    editor: &mut LineEditor,
    buffer: &str,
    cursor_byte: usize,
) -> Option<SlashCompletionResult> {
    let (command_start, command_end, prefix) = slash_command_at_cursor(buffer, cursor_byte)?;

    let mut prefix_matches = builtin_command_names()
        .into_iter()
        .filter(|name| name.starts_with(prefix))
        .collect::<Vec<_>>();

    if prefix_matches.is_empty() && prefix.is_empty() {
        prefix_matches = builtin_command_names();
    }

    if prefix_matches.len() == 1 {
        editor.replace_range(
            command_start,
            command_end,
            &format!("/{} ", prefix_matches[0]),
        );
        return Some(SlashCompletionResult {
            rendered_candidates: None,
        });
    }

    if !prefix_matches.is_empty() {
        let lcp = longest_common_prefix(&prefix_matches);
        if lcp.len() > prefix.len() {
            editor.replace_range(command_start, command_end, &format!("/{lcp}"));
            return Some(SlashCompletionResult {
                rendered_candidates: None,
            });
        }

        return Some(SlashCompletionResult {
            rendered_candidates: Some(render_slash_completion_candidates(
                prefix,
                &prefix_matches,
                false,
            )),
        });
    }

    let mut fuzzy_matches = builtin_command_names()
        .into_iter()
        .filter_map(|name| fuzzy_match_score(name, prefix).map(|score| (name, score)))
        .collect::<Vec<_>>();
    if fuzzy_matches.is_empty() {
        return None;
    }
    fuzzy_matches.sort_by(|(name_a, score_a), (name_b, score_b)| {
        score_a.cmp(score_b).then_with(|| name_a.cmp(name_b))
    });
    let fuzzy_names = fuzzy_matches
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    Some(SlashCompletionResult {
        rendered_candidates: Some(render_slash_completion_candidates(
            prefix,
            &fuzzy_names,
            true,
        )),
    })
}
