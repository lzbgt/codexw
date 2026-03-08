use anyhow::Result;

use crate::output::Output;
use crate::requests::PendingRequest;
use crate::state::summarize_text;

pub(crate) fn handle_thread_maintenance_response(
    pending: &PendingRequest,
    output: &mut Output,
) -> Result<bool> {
    match pending {
        PendingRequest::CompactThread => {
            output.line_stderr("[thread] compaction requested")?;
        }
        PendingRequest::RenameThread { name } => {
            output.line_stderr(format!("[thread] renamed to {}", summarize_text(name)))?;
        }
        PendingRequest::CleanBackgroundTerminals => {
            output.line_stderr("[thread] background terminal cleanup requested")?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
