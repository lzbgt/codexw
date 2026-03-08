use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::background_shells::BackgroundShellIntent;
use crate::orchestration_view::WorkerFilter;
use crate::orchestration_view::render_orchestration_workers;
use crate::orchestration_view::render_orchestration_workers_with_filter;
use crate::output::Output;
use crate::requests::send_clean_background_terminals;
use crate::state::AppState;
use crate::state::thread_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanTarget {
    All,
    Blockers,
    Shells,
    Services,
    Terminals,
}

pub(crate) fn handle_ps_command(
    raw_args: &str,
    args: &[&str],
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<bool> {
    let action = args.first().copied();
    if matches!(action, Some("clean")) {
        if let Some(target) = parse_clean_target(args.get(1).copied()) {
            execute_clean_target(target, cli, state, output, writer)?;
        } else {
            output
                .line_stderr("[session] usage: :ps clean [blockers|shells|services|terminals]")?;
        }
    } else if matches!(action, Some("send" | "write" | "stdin")) {
        let Some((reference, text)) = parse_ps_send_args(raw_args) else {
            output.line_stderr("[session] usage: :ps send <jobId|alias|n> <text>")?;
            return Ok(true);
        };
        let job_id = match state.background_shells.resolve_job_reference(reference) {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state
            .background_shells
            .send_input_for_operator(&job_id, text, true)
        {
            Ok(bytes_written) => output.line_stderr(format!(
                "[thread] sent {bytes_written} byte{} to background shell job {job_id}",
                if bytes_written == 1 { "" } else { "s" }
            ))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
    } else if matches!(action, Some("attach")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps attach <jobId|alias|n>")?;
            return Ok(true);
        };
        let job_id = match state.background_shells.resolve_job_reference(reference) {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state.background_shells.attach_for_operator(&job_id) {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Service Attachment", &rendered)?;
    } else if matches!(action, Some("run" | "invoke")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps run <jobId|alias|n> <recipe>")?;
            return Ok(true);
        };
        let Some(recipe) = args.get(2).copied() else {
            output.line_stderr("[session] usage: :ps run <jobId|alias|n> <recipe>")?;
            return Ok(true);
        };
        let job_id = match state.background_shells.resolve_job_reference(reference) {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state
            .background_shells
            .invoke_recipe_for_operator(&job_id, recipe)
        {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Service Recipe", &rendered)?;
    } else if matches!(action, Some("poll" | "show" | "inspect")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps poll <jobId|alias|n>")?;
            return Ok(true);
        };
        let job_id = match state.background_shells.resolve_job_reference(reference) {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state.background_shells.poll_job(&job_id, 0, 200) {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Background Shell", &rendered)?;
    } else if matches!(action, Some("alias")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps alias <jobId|n> <name>")?;
            return Ok(true);
        };
        let Some(alias) = args.get(2).copied() else {
            output.line_stderr("[session] usage: :ps alias <jobId|n> <name>")?;
            return Ok(true);
        };
        let job_id = match state.background_shells.resolve_job_reference(reference) {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state.background_shells.set_job_alias(&job_id, alias) {
            Ok(()) => output.line_stderr(format!(
                "[thread] background shell job {job_id} aliased as {alias}"
            ))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
    } else if matches!(action, Some("unalias")) {
        let Some(alias) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps unalias <name>")?;
            return Ok(true);
        };
        match state.background_shells.clear_job_alias(alias) {
            Ok(job_id) => output.line_stderr(format!(
                "[thread] removed alias {alias} from background shell job {job_id}"
            ))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
    } else if matches!(action, Some("terminate" | "stop" | "kill")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps terminate <jobId|alias|n>")?;
            return Ok(true);
        };
        let job_id = match state.background_shells.resolve_job_reference(reference) {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state.background_shells.terminate_job_for_operator(&job_id) {
            Ok(()) => output.line_stderr(format!(
                "[thread] terminated local background shell job {job_id}"
            ))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
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
            "[session] usage: :ps [blockers|agents|shells|services|terminals|attach|run|poll|send|terminate|alias|unalias|clean]",
        )?;
    }
    Ok(true)
}

fn parse_ps_send_args(raw_args: &str) -> Option<(&str, &str)> {
    let remainder = raw_args
        .trim_start()
        .strip_prefix("send")
        .or_else(|| raw_args.trim_start().strip_prefix("write"))
        .or_else(|| raw_args.trim_start().strip_prefix("stdin"))?
        .trim_start();
    let (reference, text) = remainder.split_once(char::is_whitespace)?;
    let text = text.trim_start();
    if reference.is_empty() || text.is_empty() {
        None
    } else {
        Some((reference, text))
    }
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

pub(crate) fn parse_clean_target(action: Option<&str>) -> Option<CleanTarget> {
    match action {
        None | Some("all") => Some(CleanTarget::All),
        Some("blockers") | Some("blocking") | Some("prereqs") => Some(CleanTarget::Blockers),
        Some("shells") => Some(CleanTarget::Shells),
        Some("services") => Some(CleanTarget::Services),
        Some("terminals") => Some(CleanTarget::Terminals),
        Some(_) => None,
    }
}

pub(crate) fn execute_clean_target(
    target: CleanTarget,
    cli: &Cli,
    state: &mut AppState,
    output: &mut Output,
    writer: &mut ChildStdin,
) -> Result<()> {
    let cleaned_local = match target {
        CleanTarget::All | CleanTarget::Shells => state.background_shells.terminate_all_running(),
        CleanTarget::Blockers => state
            .background_shells
            .terminate_running_by_intent(BackgroundShellIntent::Prerequisite),
        CleanTarget::Services => state
            .background_shells
            .terminate_running_by_intent(BackgroundShellIntent::Service),
        CleanTarget::Terminals => 0,
    };

    let target_label = match target {
        CleanTarget::All => "background tasks",
        CleanTarget::Blockers => "blocking prerequisite shells",
        CleanTarget::Shells => "local background shell jobs",
        CleanTarget::Services => "service shells",
        CleanTarget::Terminals => "server background terminals",
    };
    output.line_stderr(format!("[thread] cleaning {target_label}"))?;

    if cleaned_local > 0 {
        output.line_stderr(format!(
            "[thread] terminated {cleaned_local} local background shell job{}",
            if cleaned_local == 1 { "" } else { "s" }
        ))?;
    }

    match target {
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
            if !state.live_agent_tasks.is_empty() {
                output.line_stderr(
                    "[thread] active agent waits are visible in /ps blockers but are not terminable from the wrapper",
                )?;
            }
        }
        CleanTarget::Shells | CleanTarget::Services => {}
    }

    Ok(())
}
