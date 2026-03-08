use std::process::ChildStdin;

use anyhow::Result;

use crate::Cli;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellServiceIssueClass;
use crate::orchestration_view::DependencyFilter;
use crate::orchestration_view::DependencySelection;
use crate::orchestration_view::WorkerFilter;
use crate::orchestration_view::render_orchestration_actions_for_capability;
use crate::orchestration_view::render_orchestration_dependencies;
use crate::orchestration_view::render_orchestration_guidance_for_capability;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanSelection {
    pub(crate) target: CleanTarget,
    pub(crate) capability: Option<String>,
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
        match parse_clean_selection(&args[1..], ":ps clean") {
            Ok(selection) => {
                execute_clean_target(selection, cli, state, output, writer)?;
            }
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
            }
        }
    } else if matches!(action, Some("send" | "write" | "stdin")) {
        let Some((reference, text)) = parse_ps_send_args(raw_args) else {
            output.line_stderr("[session] usage: :ps send <jobId|alias|@capability|n> <text>")?;
            return Ok(true);
        };
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state
            .orchestration
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
            output.line_stderr("[session] usage: :ps attach <jobId|alias|@capability|n>")?;
            return Ok(true);
        };
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state
            .orchestration
            .background_shells
            .attach_for_operator(&job_id)
        {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Service Attachment", &rendered)?;
    } else if matches!(action, Some("wait")) {
        let Some(reference) = args.get(1).copied() else {
            output
                .line_stderr("[session] usage: :ps wait <jobId|alias|@capability|n> [timeoutMs]")?;
            return Ok(true);
        };
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let timeout_ms = match parse_ps_wait_timeout(args.get(2).copied()) {
            Ok(timeout_ms) => timeout_ms,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state
            .orchestration
            .background_shells
            .wait_ready_for_operator(&job_id, timeout_ms)
        {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Service Ready", &rendered)?;
    } else if matches!(action, Some("run" | "invoke")) {
        let Some((reference, recipe, invoke_args)) = parse_ps_run_args(raw_args) else {
            output.line_stderr(
                "[session] usage: :ps run <jobId|alias|@capability|n> <recipe> [json-args]",
            )?;
            return Ok(true);
        };
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let invoke_args = match parse_operator_recipe_args(invoke_args) {
            Ok(args) => args,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state
            .orchestration
            .background_shells
            .invoke_recipe_for_operator_with_args(&job_id, recipe, &invoke_args)
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
            output.line_stderr("[session] usage: :ps poll <jobId|alias|@capability|n>")?;
            return Ok(true);
        };
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state
            .orchestration
            .background_shells
            .poll_job(&job_id, 0, 200)
        {
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
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state
            .orchestration
            .background_shells
            .set_job_alias(&job_id, alias)
        {
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
        match state.orchestration.background_shells.clear_job_alias(alias) {
            Ok(job_id) => output.line_stderr(format!(
                "[thread] removed alias {alias} from background shell job {job_id}"
            ))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
    } else if matches!(action, Some("capabilities" | "caps" | "cap")) && args.len() > 1 {
        let selector = args[1];
        if selector.starts_with('@') {
            match state
                .orchestration
                .background_shells
                .render_single_service_capability_for_ps(selector)
            {
                Ok(rendered) => output.block_stdout("Service Capability", &rendered.join("\n"))?,
                Err(err) => output.line_stderr(format!("[session] {err}"))?,
            }
        } else {
            let issue_filter = match parse_ps_capability_issue_filter(Some(selector)) {
                Some(filter) => filter,
                None => {
                    output.line_stderr(
                        "[session] usage: :ps capabilities [@capability|healthy|missing|booting|ambiguous]",
                    )?;
                    return Ok(true);
                }
            };
            let rendered = match state
                .orchestration
                .background_shells
                .render_service_capabilities_for_ps_filtered(issue_filter)
            {
                Some(rendered) => rendered.join("\n"),
                None => "No reusable service capabilities tracked right now.".to_string(),
            };
            output.block_stdout("Service Capabilities", &rendered)?;
        }
    } else if matches!(action, Some("services")) && args.len() > 1 {
        let (issue_filter, capability_filter) = match parse_ps_service_selector(&args[1..]) {
            Ok(selection) => selection,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match state
            .orchestration
            .background_shells
            .render_service_shells_for_ps_filtered(issue_filter, capability_filter.as_deref())
        {
            Some(rendered) => rendered.join("\n"),
            None => "No service shells tracked right now.".to_string(),
        };
        output.block_stdout("Service Shells", &rendered)?;
    } else if matches!(action, Some("dependencies" | "deps")) && args.len() > 1 {
        let selection = match parse_ps_dependency_selector(&args[1..]) {
            Ok(selection) => selection,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = render_orchestration_dependencies(state, &selection);
        output.block_stdout("Dependencies", &rendered)?;
    } else if matches!(action, Some("guidance" | "guide" | "next")) && args.len() > 1 {
        let capability = match parse_ps_focus_capability(&args[1..], ":ps guidance") {
            Ok(capability) => capability,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match render_orchestration_guidance_for_capability(state, &capability) {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Guidance", &rendered)?;
    } else if matches!(
        action,
        Some("actions" | "action" | "suggest" | "suggestions")
    ) && args.len() > 1
    {
        let capability = match parse_ps_focus_capability(&args[1..], ":ps actions") {
            Ok(capability) => capability,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match render_orchestration_actions_for_capability(state, &capability) {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Actions", &rendered)?;
    } else if matches!(action, Some("terminate" | "stop" | "kill")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps terminate <jobId|alias|@capability|n>")?;
            return Ok(true);
        };
        let job_id = match state
            .orchestration
            .background_shells
            .resolve_job_reference(reference)
        {
            Ok(job_id) => job_id,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state
            .orchestration
            .background_shells
            .terminate_job_for_operator(&job_id)
        {
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
            "[session] usage: :ps [guidance [@capability]|actions [@capability]|blockers|dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]|agents|shells|services [all|ready|booting|untracked|conflicts] [@capability]|capabilities [@capability|healthy|missing|booting|ambiguous]|terminals|attach|wait|run|poll|send|terminate|alias|unalias|clean [blockers|shells|services [@capability]|terminals]]",
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

fn parse_ps_run_args(raw_args: &str) -> Option<(&str, &str, Option<&str>)> {
    let remainder = raw_args
        .trim_start()
        .strip_prefix("run")
        .or_else(|| raw_args.trim_start().strip_prefix("invoke"))?
        .trim_start();
    let (reference, remainder) = remainder.split_once(char::is_whitespace)?;
    let remainder = remainder.trim_start();
    let (recipe, arg_tail) = match remainder.split_once(char::is_whitespace) {
        Some((recipe, tail)) => (recipe, Some(tail.trim_start())),
        None => (remainder, None),
    };
    if reference.is_empty() || recipe.is_empty() {
        None
    } else {
        Some((
            reference,
            recipe,
            arg_tail.filter(|value| !value.is_empty()),
        ))
    }
}

fn parse_operator_recipe_args(
    raw_args: Option<&str>,
) -> Result<std::collections::HashMap<String, String>> {
    let Some(raw_args) = raw_args else {
        return Ok(std::collections::HashMap::new());
    };
    let value: serde_json::Value = serde_json::from_str(raw_args)
        .map_err(|err| anyhow::anyhow!("recipe args must be valid JSON object text: {err}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("recipe args must be a JSON object"))?;
    let mut args = std::collections::HashMap::with_capacity(object.len());
    for (key, value) in object {
        let rendered = match value {
            serde_json::Value::String(text) => text.clone(),
            serde_json::Value::Bool(flag) => flag.to_string(),
            serde_json::Value::Number(number) => number.to_string(),
            _ => {
                return Err(anyhow::anyhow!(
                    "recipe arg `{key}` must be a string, number, or boolean"
                ));
            }
        };
        args.insert(key.clone(), rendered);
    }
    Ok(args)
}

fn parse_ps_wait_timeout(raw: Option<&str>) -> Result<u64> {
    let Some(raw) = raw else {
        return Ok(5_000);
    };
    raw.parse::<u64>()
        .map_err(|_| anyhow::anyhow!("timeoutMs must be a non-negative integer"))
}

pub(crate) fn parse_ps_filter(action: Option<&str>) -> Option<WorkerFilter> {
    match action {
        None | Some("all") => Some(WorkerFilter::All),
        Some("guidance") | Some("guide") | Some("next") => Some(WorkerFilter::Guidance),
        Some("actions") | Some("action") | Some("suggest") | Some("suggestions") => {
            Some(WorkerFilter::Actions)
        }
        Some("blockers") | Some("blocking") | Some("prereqs") => Some(WorkerFilter::Blockers),
        Some("dependencies") | Some("deps") => Some(WorkerFilter::Dependencies),
        Some("agents") => Some(WorkerFilter::Agents),
        Some("shells") => Some(WorkerFilter::Shells),
        Some("services") => Some(WorkerFilter::Services),
        Some("capabilities") | Some("caps") | Some("cap") => Some(WorkerFilter::Capabilities),
        Some("terminals") => Some(WorkerFilter::Terminals),
        Some("clean") => None,
        Some(_) => None,
    }
}

pub(crate) fn parse_ps_focus_capability(args: &[&str], context: &str) -> Result<String, String> {
    let usage = format!("usage: {context} [@capability]");
    let [selector] = args else {
        return Err(usage);
    };
    let Some(capability) = selector.strip_prefix('@') else {
        return Err(usage);
    };
    if capability.is_empty() || !is_valid_capability_ref(capability) {
        return Err(usage);
    }
    Ok(capability.to_string())
}

pub(crate) fn parse_ps_dependency_filter(action: Option<&str>) -> Option<DependencyFilter> {
    match action {
        None | Some("all") => Some(DependencyFilter::All),
        Some("blocking") | Some("blockers") => Some(DependencyFilter::Blocking),
        Some("sidecars") | Some("sidecar") => Some(DependencyFilter::Sidecars),
        Some("missing") => Some(DependencyFilter::Missing),
        Some("booting") => Some(DependencyFilter::Booting),
        Some("ambiguous") | Some("conflicts") | Some("conflict") => {
            Some(DependencyFilter::Ambiguous)
        }
        Some("satisfied") | Some("ready") => Some(DependencyFilter::Satisfied),
        Some(_) => None,
    }
}

pub(crate) fn parse_ps_dependency_selector(
    args: &[&str],
) -> Result<DependencySelection, &'static str> {
    let mut filter = DependencyFilter::All;
    let mut capability = None;

    for arg in args.iter().copied() {
        if let Some(raw_capability) = arg.strip_prefix('@') {
            if raw_capability.is_empty() {
                return Err(
                    "usage: :ps dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]",
                );
            }
            if capability.replace(raw_capability.to_string()).is_some() {
                return Err(
                    "usage: :ps dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]",
                );
            }
            continue;
        }

        if let Some(parsed) = parse_ps_dependency_filter(Some(arg)) {
            if !matches!(filter, DependencyFilter::All) {
                return Err(
                    "usage: :ps dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]",
                );
            }
            filter = parsed;
            continue;
        }

        return Err(
            "usage: :ps dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]",
        );
    }

    Ok(DependencySelection { filter, capability })
}

pub(crate) fn parse_ps_capability_issue_filter(
    action: Option<&str>,
) -> Option<Option<crate::background_shells::BackgroundShellCapabilityIssueClass>> {
    match action {
        None | Some("all") => Some(None),
        Some("healthy") | Some("ok") => Some(Some(
            crate::background_shells::BackgroundShellCapabilityIssueClass::Healthy,
        )),
        Some("missing") => Some(Some(
            crate::background_shells::BackgroundShellCapabilityIssueClass::Missing,
        )),
        Some("booting") => Some(Some(
            crate::background_shells::BackgroundShellCapabilityIssueClass::Booting,
        )),
        Some("ambiguous") | Some("conflict") | Some("conflicts") => Some(Some(
            crate::background_shells::BackgroundShellCapabilityIssueClass::Ambiguous,
        )),
        Some(_) => None,
    }
}

pub(crate) fn parse_ps_service_issue_filter(
    action: Option<&str>,
) -> Option<Option<BackgroundShellServiceIssueClass>> {
    match action {
        None | Some("all") => Some(None),
        Some("ready") | Some("healthy") => Some(Some(BackgroundShellServiceIssueClass::Ready)),
        Some("booting") => Some(Some(BackgroundShellServiceIssueClass::Booting)),
        Some("untracked") => Some(Some(BackgroundShellServiceIssueClass::Untracked)),
        Some("ambiguous") | Some("conflict") | Some("conflicts") => {
            Some(Some(BackgroundShellServiceIssueClass::Conflicts))
        }
        Some(_) => None,
    }
}

pub(crate) fn parse_ps_service_selector(
    args: &[&str],
) -> Result<(Option<BackgroundShellServiceIssueClass>, Option<String>), &'static str> {
    let mut issue_filter = None;
    let mut capability = None;

    for arg in args.iter().copied() {
        if let Some(raw_capability) = arg.strip_prefix('@') {
            if raw_capability.is_empty()
                || !is_valid_capability_ref(raw_capability)
                || capability.replace(raw_capability.to_string()).is_some()
            {
                return Err(
                    "usage: :ps services [all|ready|booting|untracked|conflicts] [@capability]",
                );
            }
            continue;
        }

        let Some(parsed_filter) = parse_ps_service_issue_filter(Some(arg)) else {
            return Err(
                "usage: :ps services [all|ready|booting|untracked|conflicts] [@capability]",
            );
        };
        if issue_filter.replace(parsed_filter).is_some() {
            return Err(
                "usage: :ps services [all|ready|booting|untracked|conflicts] [@capability]",
            );
        }
    }

    Ok((issue_filter.unwrap_or(None), capability))
}

fn is_valid_capability_ref(raw: &str) -> bool {
    raw.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/'))
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

pub(crate) fn parse_clean_selection(
    args: &[&str],
    context: &str,
) -> Result<CleanSelection, String> {
    let usage = format!("{context} [blockers|shells|services [@capability]|terminals]");
    let Some(target) = parse_clean_target(args.first().copied()) else {
        return Err(format!("usage: {usage}"));
    };
    let capability = match args.get(1).copied() {
        None => None,
        Some(raw_capability) if matches!(target, CleanTarget::Services) => {
            let Some(raw_capability) = raw_capability.strip_prefix('@') else {
                return Err(format!("usage: {usage}"));
            };
            if raw_capability.is_empty() || !is_valid_capability_ref(raw_capability) {
                return Err(format!("usage: {usage}"));
            }
            Some(raw_capability.to_string())
        }
        Some(_) => {
            return Err(format!("usage: {usage}"));
        }
    };
    if args.len() > 2 {
        return Err(format!("usage: {usage}"));
    }
    Ok(CleanSelection { target, capability })
}

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
        CleanTarget::Blockers => state
            .orchestration
            .background_shells
            .terminate_running_by_intent(BackgroundShellIntent::Prerequisite),
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
        output.line_stderr(format!(
            "[thread] cleaning service shells for @{capability}"
        ))?;
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
