use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::orchestration_view::WorkerFilter;
use crate::orchestration_view::render_orchestration_workers;
use crate::orchestration_view::render_orchestration_workers_with_filter;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::state::AppState;
use crate::state::thread_id;

pub(crate) fn handle_ps_command(
    args: &[&str],
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let action = args.first().copied();
    if matches!(action, Some("clean")) {
        let cleaned_local = state.background_shells.terminate_all_running();
        if cli.no_experimental_api {
            output.line_stderr(
                "[thread] server background terminal cleanup requires experimental API support; restart without --no-experimental-api",
            )?;
            if cleaned_local > 0 {
                output.line_stderr(format!(
                    "[thread] terminated {cleaned_local} local background shell job{}",
                    if cleaned_local == 1 { "" } else { "s" }
                ))?;
            }
        } else {
            let current_thread_id = thread_id(state)?.to_string();
            output.line_stderr("[thread] cleaning background tasks")?;
            if cleaned_local > 0 {
                output.line_stderr(format!(
                    "[thread] terminated {cleaned_local} local background shell job{}",
                    if cleaned_local == 1 { "" } else { "s" }
                ))?;
            }
            send_clean_background_terminals(writer, state, current_thread_id)?;
        }
    } else if let Some(filter) = parse_ps_filter(action) {
        let rendered = if matches!(filter, WorkerFilter::All) {
            render_orchestration_workers(state)
        } else {
            render_orchestration_workers_with_filter(state, filter)
        };
        output.block_stdout("Workers", &rendered)?;
    } else {
        output.line_stderr(
            "[session] usage: :ps [blockers|agents|shells|services|terminals|clean]",
        )?;
    }
    Ok(true)
}

pub(crate) fn parse_ps_filter(action: Option<&str>) -> Option<WorkerFilter> {
    match action {
        None | Some("all") => Some(WorkerFilter::All),
        Some("blockers") | Some("blocking") | Some("prereqs") => Some(WorkerFilter::Blockers),
        Some("agents") => Some(WorkerFilter::Agents),
        Some("shells") => Some(WorkerFilter::Shells),
        Some("services") => Some(WorkerFilter::Services),
        Some("terminals") => Some(WorkerFilter::Terminals),
        Some("clean") => None,
        Some(_) => None,
    }
}
