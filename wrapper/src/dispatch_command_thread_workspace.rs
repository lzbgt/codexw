use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::commands::quote_if_needed;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::send_fuzzy_file_search;
use crate::state::AppState;
use crate::state::canonicalize_or_keep;
use crate::state::summarize_text;

pub(crate) fn try_handle_thread_workspace_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "mention" => {
            let query = args.join(" ");
            let query = query.trim();
            if query.is_empty() {
                editor.insert_str("@");
                true
            } else if let Ok(index) = query.parse::<usize>() {
                let Some(path) = state
                    .last_file_search_paths
                    .get(index.saturating_sub(1))
                    .cloned()
                else {
                    output.line_stderr(
                        "[session] no cached file match at that index; run /mention <query> first",
                    )?;
                    return Ok(Some(true));
                };
                let inserted = quote_if_needed(&path);
                editor.insert_str(&format!("{inserted} "));
                output.line_stderr(format!("[mention] inserted {}", summarize_text(&path)))?;
                true
            } else {
                output.line_stderr(format!("[search] files matching {}", summarize_text(query)))?;
                send_fuzzy_file_search(writer, state, resolved_cwd, query.to_string())?;
                true
            }
        }
        "diff" => {
            if let Some(diff) = state.last_turn_diff.as_deref() {
                output.block_stdout("Latest diff", diff)?;
            } else {
                output.line_stderr("[diff] no turn diff has been emitted yet")?;
            }
            true
        }
        "clear" => {
            if state.turn_running {
                output.line_stderr(
                    "[session] wait for the current turn to finish or interrupt it first",
                )?;
                true
            } else {
                output.clear_screen()?;
                output.line_stderr("[thread] creating new thread after clear")?;
                crate::requests::send_thread_start(writer, state, cli, resolved_cwd, None)?;
                true
            }
        }
        "copy" => {
            if let Some(message) = state.last_agent_message.as_deref() {
                crate::dispatch_command_utils::copy_to_clipboard(message, output)?;
            } else {
                output.line_stderr("[copy] no assistant reply is available yet")?;
            }
            true
        }
        "attach-image" | "attach" => {
            let Some(path) = args.first() else {
                output.line_stderr("[session] usage: :attach-image <path>")?;
                return Ok(Some(true));
            };
            let path = canonicalize_or_keep(path);
            state.pending_local_images.push(path.clone());
            output.line_stderr(format!("[draft] queued local image {path}"))?;
            true
        }
        "attach-url" => {
            let Some(url) = args.first() else {
                output.line_stderr("[session] usage: :attach-url <url>")?;
                return Ok(Some(true));
            };
            state.pending_remote_images.push((*url).to_string());
            output.line_stderr(format!("[draft] queued remote image {url}"))?;
            true
        }
        "clear-attachments" => {
            state.pending_local_images.clear();
            state.pending_remote_images.clear();
            output.line_stderr("[draft] cleared queued attachments")?;
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}
