use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::editor::LineEditor;
use crate::output::Output;
#[path = "dispatch_command_session_collab.rs"]
mod dispatch_command_session_collab;
#[path = "dispatch_command_session_realtime.rs"]
mod dispatch_command_session_realtime;

use crate::state::AppState;
use dispatch_command_session_collab::handle_collab_command;
use dispatch_command_session_collab::handle_plan_command;
use dispatch_command_session_realtime::handle_ps_command;
use dispatch_command_session_realtime::handle_realtime_command;

pub(crate) fn try_handle_session_mode_command(
    command: &str,
    args: &[&str],
    cli: &Cli,
    _resolved_cwd: &str,
    state: &mut AppState,
    _editor: &mut LineEditor,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<Option<bool>> {
    let result = match command {
        "auto" => {
            let Some(mode) = args.first() else {
                output.line_stderr("[session] usage: :auto on|off")?;
                return Ok(Some(true));
            };
            state.auto_continue = match *mode {
                "on" => true,
                "off" => false,
                _ => {
                    output.line_stderr("[session] usage: :auto on|off")?;
                    return Ok(Some(true));
                }
            };
            output.line_stderr(format!(
                "[auto] {}",
                if state.auto_continue {
                    "enabled"
                } else {
                    "disabled"
                }
            ))?;
            true
        }
        "collab" => handle_collab_command(args, state, output, writer)?,
        "plan" => handle_plan_command(state, output, writer)?,
        "realtime" => {
            if let Some(result) = handle_realtime_command(args, cli, state, output, writer)? {
                result
            } else {
                return Ok(None);
            }
        }
        "ps" => handle_ps_command(args, cli, state, output, writer)?,
        _ => return Ok(None),
    };

    Ok(Some(result))
}
