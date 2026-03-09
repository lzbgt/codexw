use anyhow::Result;

use crate::background_shells::BackgroundShellInteractionRecipe;
use crate::background_shells::BackgroundShellServiceIssueClass;
use crate::orchestration_view::DependencyFilter;
use crate::orchestration_view::DependencySelection;
use crate::orchestration_view::WorkerFilter;

use super::CleanSelection;
use super::CleanTarget;

pub(crate) fn parse_ps_send_args(raw_args: &str) -> Option<(&str, &str)> {
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

pub(crate) fn parse_ps_run_args(raw_args: &str) -> Option<(&str, &str, Option<&str>)> {
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

pub(crate) fn parse_ps_relabel_args(raw_args: &str) -> Option<(&str, Option<String>)> {
    let remainder = raw_args.trim_start().strip_prefix("relabel")?.trim_start();
    let (reference, label) = remainder.split_once(char::is_whitespace)?;
    let label = label.trim_start();
    if reference.is_empty() || label.is_empty() {
        return None;
    }
    let normalized = if matches!(label, "none" | "-") {
        None
    } else {
        Some(label.to_string())
    };
    Some((reference, normalized))
}

pub(crate) fn parse_ps_contract_args(
    raw_args: &str,
) -> Option<(&str, serde_json::Map<String, serde_json::Value>)> {
    let remainder = raw_args.trim_start().strip_prefix("contract")?.trim_start();
    let (reference, contract) = remainder.split_once(char::is_whitespace)?;
    let contract = contract.trim_start();
    if reference.is_empty() || contract.is_empty() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(contract).ok()?;
    let object = value.as_object()?.clone();
    Some((reference, object))
}

pub(crate) fn parse_optional_contract_field(
    value: Option<&serde_json::Value>,
    field_name: &str,
) -> Result<Option<Option<String>>, String> {
    match value {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(Some(None)),
        Some(serde_json::Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Err(format!(":ps contract `{field_name}` cannot be empty"))
            } else {
                Ok(Some(Some(trimmed.to_string())))
            }
        }
        Some(_) => Err(format!(
            ":ps contract `{field_name}` must be a string or null"
        )),
    }
}

pub(crate) fn parse_optional_contract_recipes(
    value: Option<&serde_json::Value>,
) -> Result<Option<Vec<BackgroundShellInteractionRecipe>>, String> {
    match value {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(Some(Vec::new())),
        Some(value) => {
            crate::background_shells::parse_background_shell_interaction_recipes(Some(value))
                .map(Some)
                .map_err(|err| format!(":ps contract {err}"))
        }
    }
}

pub(crate) fn parse_operator_recipe_args(
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

pub(crate) fn parse_ps_wait_timeout(raw: Option<&str>) -> Result<u64> {
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

pub(crate) fn parse_ps_capability_list(args: &[&str], usage: &str) -> Result<Vec<String>, String> {
    if args.is_empty() {
        return Err(format!("usage: {usage}"));
    }
    if args.len() == 1 && matches!(args[0], "none" | "-") {
        return Ok(Vec::new());
    }
    let mut capabilities = Vec::with_capacity(args.len());
    for raw in args {
        let Some(capability) = raw.strip_prefix('@') else {
            return Err(format!("usage: {usage}"));
        };
        if capability.is_empty() || !is_valid_capability_ref(capability) {
            return Err(format!("usage: {usage}"));
        }
        capabilities.push(capability.to_string());
    }
    capabilities.sort();
    capabilities.dedup();
    Ok(capabilities)
}

pub(crate) fn parse_ps_provide_capabilities(args: &[&str]) -> Result<Vec<String>, String> {
    parse_ps_capability_list(
        args,
        ":ps provide <jobId|alias|@capability|n> <@capability...|none>",
    )
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
