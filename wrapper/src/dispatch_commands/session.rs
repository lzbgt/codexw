use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::dispatch_command_session_collab::handle_collab_command;
use crate::dispatch_command_session_collab::handle_plan_command;
use crate::dispatch_command_session_ps::handle_ps_command;
use crate::dispatch_command_session_realtime::handle_realtime_command;
use crate::output::Output;
use crate::state::AppState;

pub(crate) fn try_handle_session_builtin_command(
    command: &str,
    raw_args: &str,
    args: &[&str],
    cli: &Cli,
    state: &mut AppState,
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
        "realtime" => match handle_realtime_command(args, cli, state, output, writer)? {
            Some(result) => return Ok(Some(result)),
            None => return Ok(None),
        },
        "ps" => handle_ps_command(raw_args, args, cli, state, output, writer)?,
        _ => return Ok(None),
    };

    Ok(Some(result))
}
