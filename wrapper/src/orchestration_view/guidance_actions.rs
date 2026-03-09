use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellInteractionParameter;
use crate::background_shells::BackgroundShellInteractionRecipe;
use crate::background_shells::BackgroundShellServiceReadiness;
use crate::background_terminals::server_background_terminal_count;
use crate::orchestration_registry::active_sidecar_agent_task_count;
use crate::orchestration_registry::active_wait_task_count;
use crate::orchestration_registry::running_service_count_by_readiness;
use crate::orchestration_registry::running_shell_count_by_intent;
use crate::state::AppState;

use super::DependencyFilter;
use super::DependencySelection;
use super::pluralize;
use super::render_orchestration_dependencies;

#[path = "guidance_actions/actions.rs"]
mod actions;

use actions::action_lines;
use actions::action_lines_for_capability;

#[cfg(test)]
pub(crate) fn orchestration_guidance_summary(state: &AppState) -> Option<String> {
    guidance_lines(state).first().cloned()
}

pub(crate) fn orchestration_next_action_summary(state: &AppState) -> Option<String> {
    action_lines(state, ActionAudience::Operator)
        .first()
        .cloned()
}

pub(crate) fn orchestration_next_action_summary_for_tool(state: &AppState) -> Option<String> {
    action_lines(state, ActionAudience::Tool).first().cloned()
}

pub(crate) fn render_orchestration_guidance(state: &AppState) -> String {
    let lines = guidance_lines(state);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Next action:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

pub(crate) fn render_orchestration_guidance_for_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = guidance_lines_for_capability(state, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Next action (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

pub(crate) fn render_orchestration_guidance_for_tool(state: &AppState) -> String {
    let lines = guidance_lines_for_tool(state);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Next action:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

pub(crate) fn render_orchestration_guidance_for_tool_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = guidance_lines_for_tool_capability(state, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Next action (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

pub(crate) fn render_orchestration_blockers_for_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    Ok(render_orchestration_dependencies(
        state,
        &DependencySelection {
            filter: DependencyFilter::Blocking,
            capability: Some(capability),
        },
    ))
}

pub(crate) fn render_orchestration_actions(state: &AppState) -> String {
    let lines = action_lines(state, ActionAudience::Operator);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Suggested actions:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

pub(crate) fn render_orchestration_actions_for_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = action_lines_for_capability(state, ActionAudience::Operator, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Suggested actions (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

pub(crate) fn render_orchestration_actions_for_tool(state: &AppState) -> String {
    let lines = action_lines(state, ActionAudience::Tool);
    if lines.is_empty() {
        String::new()
    } else {
        let mut rendered = vec!["Suggested actions:".to_string()];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        rendered.join("\n")
    }
}

pub(crate) fn render_orchestration_actions_for_tool_capability(
    state: &AppState,
    capability_ref: &str,
) -> Result<String, String> {
    let capability = normalize_capability_ref(capability_ref)?;
    let lines = action_lines_for_capability(state, ActionAudience::Tool, &capability)?;
    if lines.is_empty() {
        Ok(String::new())
    } else {
        let mut rendered = vec![format!("Suggested actions (@{capability}):")];
        for (index, line) in lines.iter().enumerate() {
            rendered.push(format!("{:>2}. {}", index + 1, line));
        }
        Ok(rendered.join("\n"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionAudience {
    Operator,
    Tool,
}

fn first_blocking_ref_for_capability(state: &AppState, capability: &str) -> Option<String> {
    state
        .orchestration
        .background_shells
        .blocking_dependency_job_refs_for_capability(capability)
        .ok()
        .and_then(|refs| refs.into_iter().next())
}

fn first_provider_ref_for_capability(state: &AppState, capability: &str) -> Option<String> {
    state
        .orchestration
        .background_shells
        .running_service_provider_refs_for_capability(capability)
        .ok()
        .and_then(|refs| refs.into_iter().next())
}

fn unique_service_ref_by_readiness(
    state: &AppState,
    readiness: BackgroundShellServiceReadiness,
) -> Option<String> {
    let refs = state
        .orchestration
        .background_shells
        .running_service_refs_by_readiness(readiness);
    if refs.len() == 1 {
        refs.into_iter().next()
    } else {
        None
    }
}

fn unique_running_service_ref(state: &AppState) -> Option<String> {
    let refs = state
        .orchestration
        .background_shells
        .running_service_snapshots()
        .into_iter()
        .map(|job| job.alias.unwrap_or(job.id))
        .collect::<Vec<_>>();
    if refs.len() == 1 {
        refs.into_iter().next()
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecipeInvocationSuggestion {
    name: String,
    operator_args_suffix: String,
    tool_args_suffix: String,
}

fn recipe_priority(recipe: &BackgroundShellInteractionRecipe) -> (usize, usize, usize, usize) {
    let normalized = recipe.name.trim().to_ascii_lowercase();
    let exact_rank = match normalized.as_str() {
        "health" => Some(0),
        "status" => Some(1),
        "ping" => Some(2),
        "ready" => Some(3),
        "check" => Some(4),
        "metrics" => Some(5),
        _ => None,
    };
    let category_rank = if exact_rank.is_some() {
        0
    } else if ["health", "status", "ping", "ready", "check", "metrics"]
        .iter()
        .any(|candidate| normalized.contains(candidate))
    {
        1
    } else {
        2
    };
    let required_count = recipe
        .parameters
        .iter()
        .filter(|parameter| parameter.required && parameter.default.is_none())
        .count();
    let total_count = recipe.parameters.len();
    (
        category_rank,
        exact_rank.unwrap_or(usize::MAX),
        required_count,
        total_count,
    )
}

fn recipe_example_args(
    parameters: &[BackgroundShellInteractionParameter],
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let mut args = serde_json::Map::new();
    for parameter in parameters {
        if let Some(default) = parameter.default.as_deref() {
            args.insert(
                parameter.name.clone(),
                serde_json::Value::String(default.to_string()),
            );
        } else if parameter.required {
            args.insert(
                parameter.name.clone(),
                serde_json::Value::String("value".to_string()),
            );
        }
    }
    if args.is_empty() { None } else { Some(args) }
}

fn executable_recipe_suggestion(
    recipes: &[BackgroundShellInteractionRecipe],
) -> Option<RecipeInvocationSuggestion> {
    let recipe = recipes
        .iter()
        .enumerate()
        .filter(|(_, recipe)| {
            !matches!(
                recipe.action,
                crate::background_shells::BackgroundShellInteractionAction::Informational
            )
        })
        .min_by_key(|(index, recipe)| {
            let (category_rank, name_rank, required_count, total_count) = recipe_priority(recipe);
            (
                category_rank,
                name_rank,
                required_count,
                total_count,
                *index,
            )
        })
        .map(|(_, recipe)| recipe)?;
    let args = recipe_example_args(&recipe.parameters);
    let operator_args_suffix = args
        .as_ref()
        .map(|value| format!(" {}", serde_json::Value::Object(value.clone())))
        .unwrap_or_default();
    let tool_args_suffix = args
        .as_ref()
        .map(|value| format!(",\"args\":{}", serde_json::Value::Object(value.clone())))
        .unwrap_or_default();
    Some(RecipeInvocationSuggestion {
        name: recipe.name.clone(),
        operator_args_suffix,
        tool_args_suffix,
    })
}

fn first_recipe_name_for_job_ref(
    state: &AppState,
    job_ref: &str,
) -> Option<RecipeInvocationSuggestion> {
    state
        .orchestration
        .background_shells
        .running_service_snapshots()
        .into_iter()
        .find(|job| job.alias.as_deref().unwrap_or(job.id.as_str()) == job_ref)
        .and_then(|job| executable_recipe_suggestion(&job.interaction_recipes))
}

fn unique_service_recipe_name_by_readiness(
    state: &AppState,
    readiness: BackgroundShellServiceReadiness,
) -> Option<RecipeInvocationSuggestion> {
    unique_service_ref_by_readiness(state, readiness)
        .and_then(|job_ref| first_recipe_name_for_job_ref(state, &job_ref))
}

fn first_recipe_name_for_capability(
    state: &AppState,
    capability: &str,
) -> Option<RecipeInvocationSuggestion> {
    state
        .orchestration
        .background_shells
        .running_service_snapshots()
        .into_iter()
        .find(|job| {
            job.service_capabilities
                .iter()
                .any(|entry| entry == capability)
        })
        .and_then(|job| executable_recipe_suggestion(&job.interaction_recipes))
}

fn operator_recipe_command(job_ref: &str, recipe: &RecipeInvocationSuggestion) -> String {
    format!(
        ":ps run {job_ref} {}{}",
        recipe.name, recipe.operator_args_suffix
    )
}

fn tool_recipe_call(job_ref: &str, recipe: &RecipeInvocationSuggestion) -> String {
    format!(
        "background_shell_invoke_recipe {{\"jobId\":\"{job_ref}\",\"recipe\":\"{}\"{}}}",
        recipe.name, recipe.tool_args_suffix
    )
}

fn unique_shell_ref_by_intent(state: &AppState, intent: BackgroundShellIntent) -> Option<String> {
    let mut refs = state
        .orchestration
        .background_shells
        .snapshots()
        .into_iter()
        .filter(|job| job.status == "running" && job.intent == intent)
        .map(|job| job.alias.unwrap_or(job.id))
        .collect::<Vec<_>>();
    refs.sort();
    refs.dedup();
    match refs.as_slice() {
        [job_ref] => Some(job_ref.clone()),
        _ => None,
    }
}

fn guidance_lines(state: &AppState) -> Vec<String> {
    let waits = active_wait_task_count(state);
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let sidecar_agents = active_sidecar_agent_task_count(state);
    let shell_sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    let blocking_capability_issues = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues();
    let capability_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflicts();
    let ready_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    let untracked_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    let terminals = server_background_terminal_count(state);

    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Missing)
    {
        return vec![
            format!(
                "A blocking shell depends on missing service capability @{}.",
                issue.capability
            ),
            match unique_running_service_ref(state) {
                Some(job_ref) => format!(
                    "Start a service shell that provides the missing reusable role, or retarget the known running service with `:ps provide {job_ref} @{}` before waiting on the blocker.",
                    issue.capability
                ),
                None => format!(
                    "Start a service shell that provides the missing reusable role, or retarget an existing running service with `:ps provide <jobId|alias|n> @{}` before waiting on the blocker.",
                    issue.capability
                ),
            },
            format!(
                "Use :ps capabilities @{} and :ps dependencies missing @{} to inspect the provider map and blocked shell.",
                issue.capability, issue.capability
            ),
        ];
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Ambiguous)
    {
        return vec![
            format!(
                "A blocking shell depends on ambiguous service capability @{}.",
                issue.capability
            ),
            "Resolve the conflicting reusable service role before relying on capability-based attachment.".to_string(),
            "Use :ps capabilities to inspect providers and consumers for that capability."
                .to_string(),
        ];
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Booting)
    {
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| format!("@{}", issue.capability));
        return vec![
            format!(
                "A blocking shell is waiting on booting service capability @{}.",
                issue.capability
            ),
            format!(
                "Use :ps services booting @{} to inspect the provider and readiness state.",
                issue.capability
            ),
            format!(
                "Use :ps wait {provider_ref} [timeoutMs] when the booting service has a readiness contract."
            ),
        ];
    }
    if prereqs > 0 {
        return vec![
            format!(
                "Main agent is blocked on {}.",
                pluralize(prereqs, "prerequisite shell", "prerequisite shells")
            ),
            "Inspect :ps blockers to identify the gating job.".to_string(),
            "Use :ps wait <jobId|alias|@capability|n> [timeoutMs] for services with readiness contracts or :ps poll <jobId|alias|@capability|n> to inspect raw output.".to_string(),
        ];
    }
    if waits > 0 {
        return vec![
            format!(
                "Main agent is blocked on {}.",
                pluralize(waits, "agent wait", "agent waits")
            ),
            "Inspect :ps blockers to see the blocking agent dependencies.".to_string(),
            "Use :multi-agents to refresh or switch into the relevant agent thread.".to_string(),
        ];
    }
    if !capability_conflicts.is_empty() {
        let conflict_count = capability_conflicts.len();
        let first = &capability_conflicts[0].0;
        return vec![
            format!(
                "{} detected across service capabilities.",
                pluralize(conflict_count, "capability conflict is", "capability conflicts are")
            ),
            format!("Resolve ambiguous reuse targets such as @{first} before relying on capability-based attachment."),
            "Use :ps capabilities to inspect the ambiguous capability map and assign more specific capabilities.".to_string(),
        ];
    }
    if ready_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        let recipe =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return vec![
            format!(
                "{} {} ready for reuse.",
                pluralize(ready_services, "service", "services"),
                if ready_services == 1 { "is" } else { "are" }
            ),
            "Use :ps services to inspect attachment metadata and available recipes.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => match recipe.as_ref() {
                    Some(recipe) => format!(
                        "Use :ps attach {job_ref} or {} to reuse the service directly.",
                        operator_recipe_command(job_ref, recipe)
                    ),
                    None => format!(
                        "Use :ps attach {job_ref} to inspect endpoint and recipe details for the ready service."
                    ),
                },
                None => "Use :ps attach <jobId|alias|@capability|n> or :ps run <jobId|alias|@capability|n> <recipe> [json-args] to reuse the service directly."
                    .to_string(),
            },
        ];
    }
    if booting_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
        return vec![
            format!(
                "{} still booting.",
                pluralize(booting_services, "service shell is", "service shells are")
            ),
            "Use :ps services to inspect readiness state and startup metadata.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use :ps wait {job_ref} [timeoutMs] when later work depends on service readiness."
                ),
                None => "Use :ps wait <jobId|alias|@capability|n> [timeoutMs] when later work depends on service readiness."
                    .to_string(),
            },
        ];
    }
    if untracked_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
        return vec![
            format!(
                "{} missing readiness or attachment metadata.",
                pluralize(untracked_services, "service shell is", "service shells are")
            ),
            "Use :ps services untracked to inspect services that still need contract metadata."
                .to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use :ps contract {job_ref} <json-object> or :ps relabel {job_ref} <label|none> to make the service reusable in place."
                ),
                None => "Use :ps contract <jobId|alias|@capability|n> <json-object> or :ps relabel <jobId|alias|@capability|n> <label|none> to make the service reusable in place."
                    .to_string(),
            },
        ];
    }
    if sidecar_agents + shell_sidecars > 0 {
        let sidecars = sidecar_agents + shell_sidecars;
        return vec![
            format!(
                "{} running without blocking the main agent.",
                pluralize(sidecars, "sidecar is", "sidecars are")
            ),
            "Continue independent work on the foreground thread.".to_string(),
            "Use :ps agents or :ps shells to inspect progress only when the result becomes relevant.".to_string(),
        ];
    }
    if terminals > 0 {
        return vec![
            format!(
                "{} still active.",
                pluralize(terminals, "server terminal is", "server terminals are")
            ),
            "Use :ps terminals to inspect them or :clean terminals to close them.".to_string(),
        ];
    }

    Vec::new()
}

fn guidance_lines_for_tool(state: &AppState) -> Vec<String> {
    let waits = active_wait_task_count(state);
    let prereqs = running_shell_count_by_intent(state, BackgroundShellIntent::Prerequisite);
    let sidecar_agents = active_sidecar_agent_task_count(state);
    let shell_sidecars = running_shell_count_by_intent(state, BackgroundShellIntent::Observation);
    let blocking_capability_issues = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues();
    let capability_conflicts = state
        .orchestration
        .background_shells
        .service_capability_conflicts();
    let ready_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Ready);
    let booting_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Booting);
    let untracked_services =
        running_service_count_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
    let terminals = server_background_terminal_count(state);

    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Missing)
    {
        let blocker_ref = first_blocking_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        let service_ref =
            unique_running_service_ref(state).unwrap_or_else(|| "<jobId|alias|n>".to_string());
        return vec![
            format!(
                "A blocking shell depends on missing service capability @{}.",
                issue.capability
            ),
            format!(
                "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{}\"]}}` to retarget a running service or start a new provider for that reusable role.",
                issue.capability
            ),
            format!(
                "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` or `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{}\"}}` to inspect or retarget the blocked shell.",
                issue.capability
            ),
        ];
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Ambiguous)
    {
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        return vec![
            format!(
                "A blocking shell depends on ambiguous service capability @{}.",
                issue.capability
            ),
            format!(
                "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role."
            ),
            format!(
                "Use `background_shell_inspect_capability {{\"capability\":\"@{}\"}}` to inspect the conflicting providers and consumers.",
                issue.capability
            ),
        ];
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Booting)
    {
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| format!("@{}", issue.capability));
        return vec![
            format!(
                "A blocking shell is waiting on booting service capability @{}.",
                issue.capability
            ),
            format!(
                "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{}\"}}` to inspect provider readiness.",
                issue.capability
            ),
            format!(
                "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` when later work depends on readiness."
            ),
        ];
    }
    if prereqs > 0 {
        return match unique_shell_ref_by_intent(state, BackgroundShellIntent::Prerequisite) {
            Some(job_ref) => vec![
                format!(
                    "Main agent is blocked on {}.",
                    pluralize(prereqs, "prerequisite shell", "prerequisite shells")
                ),
                "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the gating shell."
                    .to_string(),
                format!(
                    "Use `background_shell_poll {{\"jobId\":\"{job_ref}\"}}` to inspect the blocker output directly."
                ),
            ],
            None => vec![
                format!(
                    "Main agent is blocked on {}.",
                    pluralize(prereqs, "prerequisite shell", "prerequisite shells")
                ),
                "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the gating shells."
                    .to_string(),
                "Use `background_shell_poll {\"jobId\":\"<jobId|alias|@capability>\"}` to inspect a blocker directly."
                    .to_string(),
            ],
        };
    }
    if waits > 0 {
        return vec![
            format!(
                "Main agent is blocked on {}.",
                pluralize(waits, "agent wait", "agent waits")
            ),
            "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the active wait dependencies.".to_string(),
            "Use `orchestration_list_workers {\"filter\":\"agents\"}` to inspect cached and live agent workers.".to_string(),
        ];
    }
    if !capability_conflicts.is_empty() {
        let conflict_count = capability_conflicts.len();
        let first = &capability_conflicts[0].0;
        return vec![
            format!(
                "{} detected across service capabilities.",
                pluralize(
                    conflict_count,
                    "capability conflict is",
                    "capability conflicts are"
                )
            ),
            format!(
                "Use `background_shell_update_service {{\"jobId\":\"<jobId|alias|n>\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"<jobId|alias|n>\",\"capabilities\":null}}` to resolve ambiguous reuse targets such as @{first}."
            ),
            format!(
                "Use `background_shell_inspect_capability {{\"capability\":\"@{first}\"}}` to inspect the ambiguous capability map."
            ),
        ];
    }
    if ready_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        let recipe =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return vec![
            format!(
                "{} {} ready for reuse.",
                pluralize(ready_services, "service", "services"),
                if ready_services == 1 { "is" } else { "are" }
            ),
                "Use `background_shell_list_services {\"status\":\"ready\"}` to inspect attachment metadata and available recipes.".to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` or `{}` to reuse the ready service directly.",
                            tool_recipe_call(job_ref, recipe)
                        ),
                        None => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` to inspect endpoint and recipe details for the ready service."
                        ),
                    },
                    None => "Use `background_shell_attach {\"jobId\":\"<jobId|alias|@capability>\"}` or `background_shell_invoke_recipe {\"jobId\":\"<jobId|alias|@capability>\",\"recipe\":\"...\"}` to reuse the ready service directly.".to_string(),
                },
        ];
    }
    if booting_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
        return vec![
            format!(
                "{} still booting.",
                pluralize(booting_services, "service shell is", "service shells are")
            ),
            "Use `background_shell_list_services {\"status\":\"booting\"}` to inspect readiness state and startup metadata.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{job_ref}\",\"timeoutMs\":5000}}` when later work depends on service readiness."
                ),
                None => "Use `background_shell_wait_ready {\"jobId\":\"<jobId|alias|@capability>\",\"timeoutMs\":5000}` when later work depends on service readiness.".to_string(),
            },
        ];
    }
    if untracked_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
        return vec![
            format!(
                "{} missing readiness or attachment metadata.",
                pluralize(untracked_services, "service shell is", "service shells are")
            ),
            "Use `background_shell_list_services {\"status\":\"untracked\"}` to inspect services that still need contract metadata.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` or `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"label\":\"service-label\"}}` to make the service reusable in place."
                ),
                None => "Use `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}` or `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"label\":\"service-label\"}` to make the service reusable in place.".to_string(),
            },
        ];
    }
    if sidecar_agents + shell_sidecars > 0 {
        return vec![
            format!(
                "{} running without blocking the main agent.",
                pluralize(sidecar_agents + shell_sidecars, "sidecar is", "sidecars are")
            ),
            "Continue independent work on the foreground thread.".to_string(),
            "Use `orchestration_list_workers {\"filter\":\"agents\"}` or `orchestration_list_workers {\"filter\":\"shells\"}` to inspect progress only when the result becomes relevant.".to_string(),
        ];
    }
    if terminals > 0 {
        return vec![
            format!(
                "{} still active.",
                pluralize(terminals, "server terminal is", "server terminals are")
            ),
            "Use `orchestration_list_workers {\"filter\":\"terminals\"}` to inspect them."
                .to_string(),
        ];
    }

    Vec::new()
}

fn guidance_lines_for_capability(
    state: &AppState,
    capability: &str,
) -> Result<Vec<String>, String> {
    if let Some(issue) = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues()
        .into_iter()
        .find(|issue| issue.capability == capability)
    {
        return Ok(match issue.status {
            BackgroundShellCapabilityDependencyState::Missing => vec![
                format!("A blocking shell depends on missing service capability @{capability}."),
                match unique_running_service_ref(state) {
                    Some(job_ref) => format!(
                        "Start a provider for @{capability} or retarget the known running service with `:ps provide {job_ref} @{capability}` before waiting on the shell result."
                    ),
                    None => format!(
                        "Start a provider for @{capability} or retarget an existing running service with `:ps provide <jobId|alias|n> @{capability}` before waiting on the shell result."
                    ),
                },
                format!(
                    "Use :ps capabilities @{capability} and :ps dependencies missing @{capability} to inspect the exact blocker."
                ),
            ],
            BackgroundShellCapabilityDependencyState::Ambiguous => vec![
                format!("A blocking shell depends on ambiguous service capability @{capability}."),
                "Resolve the conflicting reusable service role before relying on capability-based attachment.".to_string(),
                format!(
                    "Use :ps capabilities @{capability} and :ps services @{capability} to inspect the conflicting providers."
                ),
            ],
            BackgroundShellCapabilityDependencyState::Booting => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!("A blocking shell is waiting on booting service capability @{capability}."),
                    format!(
                        "Use :ps services booting @{capability} to inspect the provider and readiness state."
                    ),
                    format!(
                        "Use :ps wait {provider_ref} 5000 when later work depends on readiness."
                    ),
                ]
            }
            BackgroundShellCapabilityDependencyState::Satisfied => vec![],
        });
    }

    Ok(
        match state
            .orchestration
            .background_shells
            .service_capability_issue_for_ref(capability)?
        {
            BackgroundShellCapabilityIssueClass::Missing => {
                vec![
                    format!("Reusable service capability @{capability} has no running provider."),
                    match unique_running_service_ref(state) {
                        Some(job_ref) => format!(
                            "Start a provider for @{capability} or retarget the known running service with `:ps provide {job_ref} @{capability}`."
                        ),
                        None => format!(
                            "Start a provider for @{capability} or retarget an existing running service with `:ps provide <jobId|alias|n> @{capability}`."
                        ),
                    },
                    format!(
                        "Use :ps capabilities @{capability} to confirm the missing-provider state."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Ambiguous => vec![
                format!("Reusable service capability @{capability} is ambiguous."),
                "Resolve the conflicting providers before relying on capability-based reuse."
                    .to_string(),
                format!("Use :ps capabilities @{capability} to inspect providers and consumers."),
            ],
            BackgroundShellCapabilityIssueClass::Booting => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!("Reusable service capability @{capability} is still booting."),
                    format!(
                        "Use :ps services booting @{capability} to inspect provider readiness."
                    ),
                    format!(
                        "Use :ps wait {provider_ref} 5000 when later work depends on readiness."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Untracked => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Reusable service capability @{capability} is provided by an untracked service."
                    ),
                    format!(
                        "Use :ps services untracked @{capability} to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Use :ps contract {provider_ref} <json-object> to add readyPattern or attachment metadata in place."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Healthy => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe = first_recipe_name_for_capability(state, capability);
                vec![
                    format!("Reusable service capability @{capability} is ready for reuse."),
                    format!(
                        "Use :ps attach {provider_ref} to inspect endpoint and recipe details."
                    ),
                    match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Use {} to reuse it directly.",
                            operator_recipe_command(&provider_ref, recipe)
                        ),
                        None => format!(
                            "Use :ps attach {provider_ref} to inspect endpoint and recipe details."
                        ),
                    },
                ]
            }
        },
    )
}

fn guidance_lines_for_tool_capability(
    state: &AppState,
    capability: &str,
) -> Result<Vec<String>, String> {
    if let Some(issue) = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues()
        .into_iter()
        .find(|issue| issue.capability == capability)
    {
        return Ok(match issue.status {
            BackgroundShellCapabilityDependencyState::Missing => {
                let blocker_ref = first_blocking_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "A blocking shell depends on missing service capability @{capability}."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget a running service or start a new provider for that reusable role."
                    ),
                    format!(
                        "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` or `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{capability}\"}}` to inspect or retarget the blocked shell."
                    ),
                ]
            }
            BackgroundShellCapabilityDependencyState::Ambiguous => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "A blocking shell depends on ambiguous service capability @{capability}."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role."
                    ),
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the conflicting providers and consumers."
                    ),
                ]
            }
            BackgroundShellCapabilityDependencyState::Booting => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "A blocking shell is waiting on booting service capability @{capability}."
                    ),
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect provider readiness."
                    ),
                    format!(
                        "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` when later work depends on readiness."
                    ),
                ]
            }
            BackgroundShellCapabilityDependencyState::Satisfied => vec![],
        });
    }

    Ok(
        match state
            .orchestration
            .background_shells
            .service_capability_issue_for_ref(capability)?
        {
            BackgroundShellCapabilityIssueClass::Missing => {
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!("Reusable service capability @{capability} has no running provider."),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget a running service or start a new provider."
                    ),
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to confirm the missing-provider state."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Ambiguous => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!("Reusable service capability @{capability} is ambiguous."),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role."
                    ),
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect providers and consumers."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Booting => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!("Reusable service capability @{capability} is still booting."),
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect provider readiness."
                    ),
                    format!(
                        "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` when later work depends on readiness."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Untracked => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Reusable service capability @{capability} is provided by an untracked service."
                    ),
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"untracked\",\"capability\":\"@{capability}\"}}` to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"label\":\"service-label\"}}` to add reusable contract metadata in place."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Healthy => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe = first_recipe_name_for_capability(state, capability);
                vec![
                    format!("Reusable service capability @{capability} is ready for reuse."),
                    format!(
                        "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` to inspect endpoint and recipe details."
                    ),
                    match recipe.as_ref() {
                        Some(recipe) => format!(
                            "Use `{}` to reuse it directly.",
                            tool_recipe_call(&provider_ref, recipe)
                        ),
                        None => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` to inspect endpoint and recipe details."
                        ),
                    },
                ]
            }
        },
    )
}

fn normalize_capability_ref(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().trim_start_matches('@');
    if normalized.is_empty() {
        return Err("capability selector must be a non-empty capability name".to_string());
    }
    Ok(normalized.to_string())
}
