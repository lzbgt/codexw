use anyhow::Result;

use crate::orchestration_view::WorkerFilter;
use crate::orchestration_view::render_orchestration_actions_for_capability;
use crate::orchestration_view::render_orchestration_blockers_for_capability;
use crate::orchestration_view::render_orchestration_dependencies;
use crate::orchestration_view::render_orchestration_guidance_for_capability;
use crate::orchestration_view::render_orchestration_workers;
use crate::orchestration_view::render_orchestration_workers_with_filter;
use crate::output::Output;
use crate::state::AppState;

use super::parse_ps_capability_issue_filter;
use super::parse_ps_dependency_selector;
use super::parse_ps_filter;
use super::parse_ps_focus_capability;
use super::parse_ps_service_selector;

pub(super) fn handle_ps_view_action(
    args: &[&str],
    state: &mut AppState,
    output: &mut Output,
) -> Result<bool> {
    let action = args.first().copied();
    if matches!(action, Some("capabilities" | "caps" | "cap")) && args.len() > 1 {
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
        return Ok(true);
    }
    if matches!(action, Some("services")) && args.len() > 1 {
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
        return Ok(true);
    }
    if matches!(action, Some("dependencies" | "deps")) && args.len() > 1 {
        let selection = match parse_ps_dependency_selector(&args[1..]) {
            Ok(selection) => selection,
            Err(err) => {
                output.line_stderr(format!("[session] {err}"))?;
                return Ok(true);
            }
        };
        let rendered = render_orchestration_dependencies(state, &selection);
        output.block_stdout("Dependencies", &rendered)?;
        return Ok(true);
    }
    if matches!(action, Some("guidance" | "guide" | "next")) && args.len() > 1 {
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
        return Ok(true);
    }
    if matches!(
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
        return Ok(true);
    }
    if matches!(action, Some("blockers" | "blocking" | "prereqs")) && args.len() > 1 {
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
        return Ok(true);
    }
    if let Some(filter) = parse_ps_filter(action) {
        let rendered = if matches!(filter, WorkerFilter::All) {
            render_orchestration_workers(state)
        } else {
            render_orchestration_workers_with_filter(state, filter)
        };
        output.block_stdout("Workers", &rendered)?;
        return Ok(true);
    }
    Ok(false)
}
