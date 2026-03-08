use anyhow::Result;

use crate::output::Output;
use crate::state::AppState;
use crate::state::canonicalize_or_keep;

pub(crate) fn handle_thread_draft_command(
    command: &str,
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<Option<bool>> {
    let result = match command {
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
