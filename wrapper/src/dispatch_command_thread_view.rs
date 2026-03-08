use std::process::ChildStdin;

use anyhow::Result;

use crate::output::Output;
use crate::requests::send_fuzzy_file_search;
use crate::state::AppState;
use crate::state::summarize_text;

pub(crate) fn handle_thread_view_command(
    command: &str,
    args: &[&str],
    resolved_cwd: &str,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "mention" => {
            let query = args.join(" ");
            let query = query.trim();
            if query.is_empty() || query.parse::<usize>().is_ok() {
                return Ok(None);
            }
            output.line_stderr(format!("[search] files matching {}", summarize_text(query)))?;
            send_fuzzy_file_search(writer, state, resolved_cwd, query.to_string())?;
            true
        }
        "diff" => {
            if let Some(diff) = state.last_turn_diff.as_deref() {
                output.block_stdout("Latest diff", diff)?;
            } else {
                output.line_stderr("[diff] no turn diff has been emitted yet")?;
            }
            true
        }
        "copy" => {
            if let Some(message) = state.last_agent_message.as_deref() {
                crate::dispatch_command_utils::copy_to_clipboard(message, output)?;
            } else {
                output.line_stderr("[copy] no assistant reply is available yet")?;
            }
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}

pub(crate) fn resolve_cached_mention(args: &[&str], state: &AppState) -> Option<String> {
    let query = args.join(" ");
    let query = query.trim();
    let index = query.parse::<usize>().ok()?;
    state
        .last_file_search_paths
        .get(index.saturating_sub(1))
        .cloned()
}
