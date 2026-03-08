use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
use crate::requests::send_load_config;
use crate::session_realtime_status::render_realtime_status;
use crate::session_snapshot_overview::render_status_overview;
use crate::session_snapshot_runtime::render_status_runtime;
use crate::state::AppState;
use crate::status_views::render_permissions_snapshot;
use crate::transcript_render::render_pending_attachments;

pub(crate) fn try_handle_session_status_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "attachments" => {
            if state.pending_local_images.is_empty() && state.pending_remote_images.is_empty() {
                output.line_stderr("[draft] no queued attachments")?;
            } else {
                let rendered = render_pending_attachments(
                    &state.pending_local_images,
                    &state.pending_remote_images,
                );
                output.block_stdout("Queued attachments", &rendered)?;
            }
            true
        }
        "approvals" | "permissions" => {
            output.block_stdout("Permissions", &render_permissions_snapshot(cli))?;
            true
        }
        "status" | "statusline" => {
            let mut lines = render_status_overview(cli, resolved_cwd, state);
            lines.extend(render_status_runtime(cli, state));
            output.block_stdout("Status", &lines.join("\n"))?;
            true
        }
        "settings" | "debug-config" => {
            output.line_stderr("[session] loading effective config")?;
            send_load_config(writer, state)?;
            true
        }
        "realtime" if args.is_empty() || matches!(args[0], "status" | "show") => {
            output.block_stdout("Realtime", &render_realtime_status(state))?;
            true
        }
        _ => return Ok(None),
    };

    Ok(Some(result))
}
