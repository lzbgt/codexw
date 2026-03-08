use crate::Cli;
#[path = "session_snapshot_overview.rs"]
mod session_snapshot_overview;
#[path = "session_snapshot_runtime.rs"]
mod session_snapshot_runtime;

use crate::state::AppState;

pub(crate) fn render_status_snapshot(cli: &Cli, resolved_cwd: &str, state: &AppState) -> String {
    let mut lines = session_snapshot_overview::render_status_overview(cli, resolved_cwd, state);
    lines.extend(session_snapshot_runtime::render_status_runtime(cli, state));
    lines.join("\n")
}
