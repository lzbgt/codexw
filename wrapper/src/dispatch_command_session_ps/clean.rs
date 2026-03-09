use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::background_shells::BackgroundShellIntent;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::state::AppState;
use crate::state::thread_id;

use super::CleanSelection;
use super::CleanTarget;

pub(crate) fn execute_clean_target(
    selection: CleanSelection,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    let cleaned_local = match selection.target {
        CleanTarget::All | CleanTarget::Shells => state
            .orchestration
            .background_shells
            .terminate_all_running(),
        CleanTarget::Blockers => {
            if let Some(capability) = selection.capability.as_deref() {
                match state
                    .orchestration
                    .background_shells
                    .terminate_running_blockers_by_capability(capability)
                {
                    Ok(cleaned_local) => cleaned_local,
                    Err(err) => {
                        output.line_stderr(format!("[session] {err}"))?;
                        return Ok(());
                    }
                }
            } else {
                state
                    .orchestration
                    .background_shells
                    .terminate_running_by_intent(BackgroundShellIntent::Prerequisite)
            }
        }
        CleanTarget::Services => {
            if let Some(capability) = selection.capability.as_deref() {
                match state
                    .orchestration
                    .background_shells
                    .terminate_running_services_by_capability(capability)
                {
                    Ok(cleaned_local) => cleaned_local,
                    Err(err) => {
                        output.line_stderr(format!("[session] {err}"))?;
                        return Ok(());
                    }
                }
            } else {
                state
                    .orchestration
                    .background_shells
                    .terminate_running_by_intent(BackgroundShellIntent::Service)
            }
        }
        CleanTarget::Terminals => 0,
    };

    let target_label = match selection.target {
        CleanTarget::All => "background tasks",
        CleanTarget::Blockers => "blocking prerequisite shells",
        CleanTarget::Shells => "local background shell jobs",
        CleanTarget::Services => "service shells",
        CleanTarget::Terminals => "server background terminals",
    };
    if let Some(capability) = selection.capability.as_deref() {
        let scope_label = match selection.target {
            CleanTarget::Blockers => "blocking prerequisite shells",
            CleanTarget::Services => "service shells",
            _ => unreachable!(),
        };
        output.line_stderr(format!("[thread] cleaning {scope_label} for @{capability}"))?;
    } else {
        output.line_stderr(format!("[thread] cleaning {target_label}"))?;
    }

    if cleaned_local > 0 {
        output.line_stderr(format!(
            "[thread] terminated {cleaned_local} local background shell job{}",
            if cleaned_local == 1 { "" } else { "s" }
        ))?;
    }

    match selection.target {
        CleanTarget::All | CleanTarget::Terminals => {
            if cli.no_experimental_api {
                output.line_stderr(
                    "[thread] server background terminal cleanup requires experimental API support; restart without --no-experimental-api",
                )?;
            } else {
                let current_thread_id = thread_id(state)?.to_string();
                send_clean_background_terminals(writer, state, current_thread_id)?;
            }
        }
        CleanTarget::Blockers => {
            if !state.orchestration.live_agent_tasks.is_empty() {
                output.line_stderr(
                    "[thread] active agent waits are visible in /ps blockers but are not terminable from the wrapper",
                )?;
            }
        }
        CleanTarget::Shells | CleanTarget::Services => {}
    }

    Ok(())
}
