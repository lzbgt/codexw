use crate::background_shells::BackgroundShellServiceIssueClass;
use crate::orchestration_view::DependencyFilter;
use crate::orchestration_view::DependencySelection;
use crate::orchestration_view::WorkerFilter;

use super::super::CleanSelection;
use super::super::CleanTarget;

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
    if capability.is_empty() || !super::is_valid_capability_ref(capability) {
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
    let usage = "usage: :ps dependencies [all|blocking|sidecars|missing|booting|ambiguous|satisfied] [@capability]";
    let mut filter = DependencyFilter::All;
    let mut capability = None;

    for arg in args.iter().copied() {
        if let Some(raw_capability) = arg.strip_prefix('@') {
            if raw_capability.is_empty() {
                return Err(usage);
            }
            if capability.replace(raw_capability.to_string()).is_some() {
                return Err(usage);
            }
            continue;
        }

        if let Some(parsed) = parse_ps_dependency_filter(Some(arg)) {
            if !matches!(filter, DependencyFilter::All) {
                return Err(usage);
            }
            filter = parsed;
            continue;
        }

        return Err(usage);
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
        Some("untracked") => Some(Some(
            crate::background_shells::BackgroundShellCapabilityIssueClass::Untracked,
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
    let usage = "usage: :ps services [all|ready|booting|untracked|conflicts] [@capability]";
    let mut issue_filter = None;
    let mut capability = None;

    for arg in args.iter().copied() {
        if let Some(raw_capability) = arg.strip_prefix('@') {
            if raw_capability.is_empty()
                || !super::is_valid_capability_ref(raw_capability)
                || capability.replace(raw_capability.to_string()).is_some()
            {
                return Err(usage);
            }
            continue;
        }

        let Some(parsed_filter) = parse_ps_service_issue_filter(Some(arg)) else {
            return Err(usage);
        };
        if issue_filter.replace(parsed_filter).is_some() {
            return Err(usage);
        }
    }

    Ok((issue_filter.unwrap_or(None), capability))
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
    let usage =
        format!("{context} [blockers [@capability]|shells|services [@capability]|terminals]");
    let Some(target) = parse_clean_target(args.first().copied()) else {
        return Err(format!("usage: {usage}"));
    };
    let capability = match args.get(1).copied() {
        None => None,
        Some(raw_capability) if matches!(target, CleanTarget::Blockers | CleanTarget::Services) => {
            let Some(raw_capability) = raw_capability.strip_prefix('@') else {
                return Err(format!("usage: {usage}"));
            };
            if raw_capability.is_empty() || !super::is_valid_capability_ref(raw_capability) {
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
