use std::process::ChildStdin;

use anyhow::Result;

#[path = "dispatch_command_session_ps/clean.rs"]
mod clean;
#[path = "dispatch_command_session_ps/parse.rs"]
mod parse;

use crate::Cli;
use crate::orchestration_view::WorkerFilter;
use crate::orchestration_view::render_orchestration_actions_for_capability;
use crate::orchestration_view::render_orchestration_blockers_for_capability;
use crate::orchestration_view::render_orchestration_dependencies;
use crate::orchestration_view::render_orchestration_guidance_for_capability;
use crate::orchestration_view::render_orchestration_workers;
use crate::orchestration_view::render_orchestration_workers_with_filter;
use crate::output::Output;
use crate::state::AppState;

pub(crate) use self::clean::execute_clean_target;
pub(crate) use self::parse::parse_clean_selection;
#[cfg(test)]
pub(crate) use self::parse::parse_clean_target;
pub(crate) use self::parse::parse_operator_recipe_args;
pub(crate) use self::parse::parse_optional_contract_field;
pub(crate) use self::parse::parse_optional_contract_recipes;
pub(crate) use self::parse::parse_ps_capability_issue_filter;
pub(crate) use self::parse::parse_ps_capability_list;
pub(crate) use self::parse::parse_ps_contract_args;
#[cfg(test)]
pub(crate) use self::parse::parse_ps_dependency_filter;
pub(crate) use self::parse::parse_ps_dependency_selector;
pub(crate) use self::parse::parse_ps_filter;
pub(crate) use self::parse::parse_ps_focus_capability;
pub(crate) use self::parse::parse_ps_provide_capabilities;
pub(crate) use self::parse::parse_ps_relabel_args;
pub(crate) use self::parse::parse_ps_run_args;
pub(crate) use self::parse::parse_ps_send_args;
#[cfg(test)]
pub(crate) use self::parse::parse_ps_service_issue_filter;
pub(crate) use self::parse::parse_ps_service_selector;
pub(crate) use self::parse::parse_ps_wait_timeout;

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
            output.line_stderr("[session] usage: :ps alias <jobId|alias|@capability|n> <name>")?;
            return Ok(true);
        };
        let Some(alias) = args.get(2).copied() else {
            output.line_stderr("[session] usage: :ps alias <jobId|alias|@capability|n> <name>")?;
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
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr("[session] usage: :ps unalias <name|jobId|alias|@capability|n>")?;
            return Ok(true);
        };
        match state
            .orchestration
            .background_shells
            .clear_job_alias(reference)
        {
            Ok(job_id) => output.line_stderr(format!(
                "[thread] removed alias {reference} from background shell job {job_id}"
            ))?,
            Err(alias_err) => {
                match state
                    .orchestration
                    .background_shells
                    .resolve_job_reference(reference)
                {
                    Ok(job_id) => {
                        if let Err(err) = state
                            .orchestration
                            .background_shells
                            .clear_job_alias_for_job(&job_id)
                        {
                            output.line_stderr(format!("[session] {err}"))?;
                        } else {
                            output.line_stderr(format!(
                                "[thread] cleared alias for background shell job {job_id}"
                            ))?;
                        }
                    }
                    Err(_) => output.line_stderr(format!("[session] {alias_err}"))?,
                }
            }
        }
    } else if matches!(action, Some("provide")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr(
                "[session] usage: :ps provide <jobId|alias|@capability|n> <@capability...|none>",
            )?;
            return Ok(true);
        };
        let capabilities = match parse_ps_provide_capabilities(&args[2..]) {
            Ok(capabilities) => capabilities,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state
            .orchestration
            .background_shells
            .update_service_capabilities_for_operator(reference, &capabilities)
        {
            Ok(summary) => output.line_stderr(format!("[thread] {summary}"))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
    } else if matches!(action, Some("depend" | "requires")) {
        let Some(reference) = args.get(1).copied() else {
            output.line_stderr(
                "[session] usage: :ps depend <jobId|alias|@capability|n> <@capability...|none>",
            )?;
            return Ok(true);
        };
        let capabilities = match parse_ps_capability_list(
            &args[2..],
            ":ps depend <jobId|alias|@capability|n> <@capability...|none>",
        ) {
            Ok(capabilities) => capabilities,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        match state
            .orchestration
            .background_shells
            .update_dependency_capabilities_for_operator(reference, &capabilities)
        {
            Ok(summary) => output.line_stderr(format!("[thread] {summary}"))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
    } else if matches!(action, Some("contract")) {
        let Some((reference, contract)) = parse_ps_contract_args(raw_args) else {
            output.line_stderr(
                "[session] usage: :ps contract <jobId|alias|@capability|n> <json-object>",
            )?;
            return Ok(true);
        };
        let protocol = match parse_optional_contract_field(contract.get("protocol"), "protocol") {
            Ok(value) => value,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let endpoint = match parse_optional_contract_field(contract.get("endpoint"), "endpoint") {
            Ok(value) => value,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let attach_hint =
            match parse_optional_contract_field(contract.get("attachHint"), "attachHint") {
                Ok(value) => value,
                Err(err) => {
                    output.line_stderr(format!("[session] {err}"))?;
                    return Ok(true);
                }
            };
        let ready_pattern =
            match parse_optional_contract_field(contract.get("readyPattern"), "readyPattern") {
                Ok(value) => value,
                Err(err) => {
                    output.line_stderr(format!("[session] {err}"))?;
                    return Ok(true);
                }
            };
        let recipes = match parse_optional_contract_recipes(contract.get("recipes")) {
            Ok(value) => value,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        if protocol.is_none()
            && endpoint.is_none()
            && attach_hint.is_none()
            && ready_pattern.is_none()
            && recipes.is_none()
        {
            output.line_stderr(
                "[session] :ps contract requires at least one of `protocol`, `endpoint`, `attachHint`, `readyPattern`, or `recipes`",
            )?;
            return Ok(true);
        }
        match state
            .orchestration
            .background_shells
            .update_service_contract_for_operator(
                reference,
                protocol,
                endpoint,
                attach_hint,
                ready_pattern,
                recipes,
            ) {
            Ok(summary) => output.line_stderr(format!("[thread] {summary}"))?,
            Err(err) => output.line_stderr(format!("[session] {err}"))?,
        }
    } else if matches!(action, Some("relabel")) {
        let Some((reference, label)) = parse_ps_relabel_args(raw_args) else {
            output.line_stderr(
                "[session] usage: :ps relabel <jobId|alias|@capability|n> <label|none>",
            )?;
            return Ok(true);
        };
        match state
            .orchestration
            .background_shells
            .update_service_label_for_operator(reference, label)
        {
            Ok(summary) => output.line_stderr(format!("[thread] {summary}"))?,
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
                        "[session] usage: :ps capabilities [@capability|healthy|missing|booting|untracked|ambiguous]",
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
    } else if matches!(action, Some("blockers" | "blocking" | "prereqs")) && args.len() > 1 {
        let capability = match parse_ps_focus_capability(&args[1..], ":ps blockers") {
            Ok(capability) => capability,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = match render_orchestration_blockers_for_capability(state, &capability) {
            Ok(rendered) => rendered,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        output.block_stdout("Blockers", &rendered)?;
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
            "[session] usage: :ps [guidance [@capability]|actions [@capability]|blockers [@capability]|dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]|agents|shells|services [all|ready|booting|untracked|conflicts] [@capability]|capabilities [@capability|healthy|missing|booting|untracked|ambiguous]|terminals|attach|wait|run|poll|send|terminate|alias|unalias|provide <jobId|alias|@capability|n> <@capability...|none>|depend <jobId|alias|@capability|n> <@capability...|none>|contract <jobId|alias|@capability|n> <json-object>|relabel <jobId|alias|@capability|n> <label|none>|clean [blockers [@capability]|shells|services [@capability]|terminals]]",
        )?;
    }
    Ok(true)
}
