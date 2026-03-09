use crate::background_shells::BackgroundShellCapabilityDependencyState;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::background_shells::BackgroundShellIntent;
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

fn first_recipe_name_for_job_ref(state: &AppState, job_ref: &str) -> Option<String> {
    state
        .orchestration
        .background_shells
        .running_service_snapshots()
        .into_iter()
        .find(|job| job.alias.as_deref().unwrap_or(job.id.as_str()) == job_ref)
        .and_then(|job| {
            job.interaction_recipes
                .iter()
                .find(|recipe| {
                    !matches!(
                        recipe.action,
                        crate::background_shells::BackgroundShellInteractionAction::Informational
                    )
                })
                .map(|recipe| recipe.name.clone())
        })
}

fn unique_service_recipe_name_by_readiness(
    state: &AppState,
    readiness: BackgroundShellServiceReadiness,
) -> Option<String> {
    unique_service_ref_by_readiness(state, readiness)
        .and_then(|job_ref| first_recipe_name_for_job_ref(state, &job_ref))
}

fn first_recipe_name_for_capability(state: &AppState, capability: &str) -> Option<String> {
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
        .and_then(|job| {
            job.interaction_recipes
                .iter()
                .find(|recipe| {
                    !matches!(
                        recipe.action,
                        crate::background_shells::BackgroundShellInteractionAction::Informational
                    )
                })
                .map(|recipe| recipe.name.clone())
        })
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
                "Use /ps capabilities @{} and /ps dependencies missing @{} to inspect the provider map and blocked shell.",
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
            "Use /ps capabilities to inspect providers and consumers for that capability."
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
                "Use /ps services booting @{} to inspect the provider and readiness state.",
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
            "Inspect /ps blockers to identify the gating job.".to_string(),
            "Use :ps wait <jobId|alias|@capability|n> [timeoutMs] for services with readiness contracts or :ps poll <jobId|alias|@capability|n> to inspect raw output.".to_string(),
        ];
    }
    if waits > 0 {
        return vec![
            format!(
                "Main agent is blocked on {}.",
                pluralize(waits, "agent wait", "agent waits")
            ),
            "Inspect /ps blockers to see the blocking agent dependencies.".to_string(),
            "Use /multi-agents to refresh or switch into the relevant agent thread.".to_string(),
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
            "Use /ps capabilities to inspect the ambiguous capability map and assign more specific capabilities.".to_string(),
        ];
    }
    if ready_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        let recipe_name =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return vec![
            format!(
                "{} {} ready for reuse.",
                pluralize(ready_services, "service", "services"),
                if ready_services == 1 { "is" } else { "are" }
            ),
            "Use /ps services to inspect attachment metadata and available recipes.".to_string(),
            match provider_ref.as_deref() {
                Some(job_ref) => match recipe_name.as_deref() {
                    Some(recipe) => format!(
                        "Use :ps attach {job_ref} or :ps run {job_ref} {recipe} [json-args] to reuse the service directly."
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
            "Use /ps services to inspect readiness state and startup metadata.".to_string(),
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
            "Use /ps services untracked to inspect services that still need contract metadata."
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
            "Use /ps agents or /ps shells to inspect progress only when the result becomes relevant.".to_string(),
        ];
    }
    if terminals > 0 {
        return vec![
            format!(
                "{} still active.",
                pluralize(terminals, "server terminal is", "server terminals are")
            ),
            "Use /ps terminals to inspect them or /clean terminals to close them.".to_string(),
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
        let recipe_name =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return vec![
            format!(
                "{} {} ready for reuse.",
                pluralize(ready_services, "service", "services"),
                if ready_services == 1 { "is" } else { "are" }
            ),
                "Use `background_shell_list_services {\"status\":\"ready\"}` to inspect attachment metadata and available recipes.".to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => match recipe_name.as_deref() {
                        Some(recipe) => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` or `background_shell_invoke_recipe {{\"jobId\":\"{job_ref}\",\"recipe\":\"{recipe}\"}}` to reuse the ready service directly."
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
                    "Use /ps capabilities @{capability} and /ps dependencies missing @{capability} to inspect the exact blocker."
                ),
            ],
            BackgroundShellCapabilityDependencyState::Ambiguous => vec![
                format!("A blocking shell depends on ambiguous service capability @{capability}."),
                "Resolve the conflicting reusable service role before relying on capability-based attachment.".to_string(),
                format!(
                    "Use /ps capabilities @{capability} and /ps services @{capability} to inspect the conflicting providers."
                ),
            ],
            BackgroundShellCapabilityDependencyState::Booting => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!("A blocking shell is waiting on booting service capability @{capability}."),
                    format!(
                        "Use /ps services booting @{capability} to inspect the provider and readiness state."
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
                        "Use /ps capabilities @{capability} to confirm the missing-provider state."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Ambiguous => vec![
                format!("Reusable service capability @{capability} is ambiguous."),
                "Resolve the conflicting providers before relying on capability-based reuse."
                    .to_string(),
                format!("Use /ps capabilities @{capability} to inspect providers and consumers."),
            ],
            BackgroundShellCapabilityIssueClass::Booting => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!("Reusable service capability @{capability} is still booting."),
                    format!(
                        "Use /ps services booting @{capability} to inspect provider readiness."
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
                        "Use /ps services untracked @{capability} to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Use :ps contract {provider_ref} <json-object> to add readyPattern or attachment metadata in place."
                    ),
                ]
            }
            BackgroundShellCapabilityIssueClass::Healthy => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe_name = first_recipe_name_for_capability(state, capability);
                vec![
                    format!("Reusable service capability @{capability} is ready for reuse."),
                    format!(
                        "Use /ps attach {provider_ref} to inspect endpoint and recipe details."
                    ),
                    format!(
                        "Use :ps run {provider_ref} {} [json-args] to reuse it directly.",
                        recipe_name.as_deref().unwrap_or("<recipe>")
                    ),
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
                let recipe_name = first_recipe_name_for_capability(state, capability);
                vec![
                    format!("Reusable service capability @{capability} is ready for reuse."),
                    format!(
                        "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` to inspect endpoint and recipe details."
                    ),
                    format!(
                        "Use `background_shell_invoke_recipe {{\"jobId\":\"{provider_ref}\",\"recipe\":\"{}\"}}` to reuse it directly.",
                        recipe_name.as_deref().unwrap_or("...")
                    ),
                ]
            }
        },
    )
}

fn action_lines(state: &AppState, audience: ActionAudience) -> Vec<String> {
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
        return match audience {
            ActionAudience::Operator => vec![
                format!(
                    "Run `:ps capabilities @{}` to inspect the missing provider map.",
                    issue.capability
                ),
                format!(
                    "Run `:ps provide {service_ref} @{}` to retarget an existing running service, or start a new provider for that role.",
                    issue.capability
                ),
                format!(
                    "Run `:ps depend {blocker_ref} <@capability...|none>` to retarget the blocked shell if it should wait on a different reusable role."
                ),
                format!(
                    "Run `:ps dependencies missing @{}` to inspect the blocked dependency edges.",
                    issue.capability
                ),
                format!(
                    "If the blocked shell is no longer needed, run `:clean blockers @{}`.",
                    issue.capability
                ),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{}\"}}` to inspect the missing provider map.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{}\"]}}` to retarget an existing running service, or start a new provider for that capability.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` to retarget the blocked shell if it should depend on a different reusable role."
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{}\"}}` to inspect the blocked dependency edges.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"blockers\",\"capability\":\"@{}\"}}` to abandon the blocking prerequisite shells if they are no longer needed.",
                    issue.capability
                ),
            ],
        };
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Ambiguous)
    {
        let blocker_ref = first_blocking_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        return match audience {
            ActionAudience::Operator => vec![
                format!(
                    "Run `:ps capabilities @{}` to inspect the ambiguous provider set.",
                    issue.capability
                ),
                format!(
                    "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{} on one running provider before falling back to cleanup.",
                    issue.capability
                ),
                format!(
                    "Run `:ps depend {blocker_ref} <@capability...|none>` if the blocked shell should be retargeted to a different dependency role."
                ),
                format!(
                    "Run `:clean services @{}` to clear the conflicting reusable role in one step.",
                    issue.capability
                ),
                format!(
                    "Run `:ps services @{}` to inspect the remaining providers.",
                    issue.capability
                ),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{}\"}}` to inspect the ambiguous provider set.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role before falling back to cleanup."
                ),
                format!(
                    "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` if the blocked shell should be retargeted to a different dependency role."
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{}\"}}` to clear the conflicting reusable role in one step.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_list_services {{\"capability\":\"@{}\"}}` to inspect the remaining providers.",
                    issue.capability
                ),
            ],
        };
    }
    if let Some(issue) = blocking_capability_issues
        .iter()
        .find(|issue| issue.status == BackgroundShellCapabilityDependencyState::Booting)
    {
        let provider_ref = first_provider_ref_for_capability(state, &issue.capability)
            .unwrap_or_else(|| format!("@{}", issue.capability));
        return match audience {
            ActionAudience::Operator => vec![
                format!(
                    "Run `:ps services booting @{}` to inspect the booting provider state.",
                    issue.capability
                ),
                format!("Run `:ps wait {provider_ref} 5000` to wait on the capability provider."),
                format!(
                    "Run `:ps dependencies booting @{}` to keep the dependency view focused.",
                    issue.capability
                ),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{}\"}}` to inspect the booting provider state.",
                    issue.capability
                ),
                format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` to wait on the capability provider."
                ),
                format!(
                    "Use `orchestration_list_dependencies {{\"filter\":\"booting\",\"capability\":\"@{}\"}}` to keep the dependency view focused.",
                    issue.capability
                ),
            ],
        };
    }
    if prereqs > 0 {
        let blocker_ref = unique_shell_ref_by_intent(state, BackgroundShellIntent::Prerequisite);
        return match audience {
            ActionAudience::Operator => {
                let mut lines = vec![
                    "Run `:ps blockers` to inspect the gating shell or wait dependency."
                        .to_string(),
                ];
                if let Some(job_ref) = blocker_ref.as_deref() {
                    lines.push(format!(
                        "Run `:ps poll {job_ref}` to inspect the blocking shell output directly."
                    ));
                } else {
                    lines.push(
                        "Run `:ps poll <jobId|alias|@capability|n>` on the blocker you care about."
                            .to_string(),
                    );
                }
                lines.push(
                    "Run `:clean blockers` to abandon the current blocking prerequisite work."
                        .to_string(),
                );
                lines
            }
            ActionAudience::Tool => {
                let mut lines = vec![
                    "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the gating shell or wait dependency.".to_string(),
                ];
                if let Some(job_ref) = blocker_ref.as_deref() {
                    lines.push(format!(
                        "Use `background_shell_poll {{\"jobId\":\"{job_ref}\"}}` to inspect the blocking shell output directly."
                    ));
                } else {
                    lines.push(
                        "Use `background_shell_poll {\"jobId\":\"bg-...\"}` on the blocker you care about."
                            .to_string(),
                    );
                }
                lines.push(
                    "Use `background_shell_clean {\"scope\":\"blockers\"}` to abandon the current blocking prerequisite work.".to_string(),
                );
                lines
            }
        };
    }
    if waits > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps blockers` to inspect the active wait dependencies.".to_string(),
                "Run `:multi-agents` to refresh spawned agent threads.".to_string(),
                "Run `:resume <n>` to switch into the agent thread that matters.".to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"blockers\"}` to inspect the active wait dependencies.".to_string(),
                "Use `orchestration_list_workers {\"filter\":\"agents\"}` to inspect cached and live agent workers.".to_string(),
                "Continue foreground work until one of the waiting agent results becomes critical.".to_string(),
            ],
        };
    }
    if let Some((capability, _)) = capability_conflicts.first() {
        let provider_ref = first_provider_ref_for_capability(state, capability)
            .unwrap_or_else(|| "<jobId|alias|n>".to_string());
        return match audience {
            ActionAudience::Operator => vec![
                format!("Run `:ps capabilities @{capability}` to inspect providers and consumers."),
                format!(
                    "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{capability} on one running provider before falling back to cleanup."
                ),
                format!(
                    "Run `:clean services @{capability}` to clear the ambiguous reusable role."
                ),
                format!("Run `:ps services @{capability}` to verify the surviving providers."),
            ],
            ActionAudience::Tool => vec![
                format!(
                    "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect providers and consumers."
                ),
                format!(
                    "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role before falling back to cleanup."
                ),
                format!(
                    "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the ambiguous reusable role."
                ),
                format!(
                    "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to verify the surviving providers."
                ),
            ],
        };
    }
    if ready_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        let recipe_name =
            unique_service_recipe_name_by_readiness(state, BackgroundShellServiceReadiness::Ready);
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps services ready` to inspect reusable service metadata.".to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Run `:ps attach {job_ref}` to inspect endpoint and recipe details."
                    ),
                    None => "Run `:ps attach <jobId|alias|@capability|n>` to inspect endpoint and recipe details."
                        .to_string(),
                },
                match provider_ref.as_deref() {
                    Some(job_ref) => match recipe_name.as_deref() {
                        Some(recipe) => format!(
                            "Run `:ps attach {job_ref}` or `:ps run {job_ref} {recipe} [json-args]` to reuse the ready service directly."
                        ),
                        None => format!(
                            "Run `:ps attach {job_ref}` to inspect endpoint and recipe details for the ready service."
                        ),
                    },
                    None => "Run `:ps run <jobId|alias|@capability|n> <recipe> [json-args]` to reuse the ready service directly."
                        .to_string(),
                },
            ],
            ActionAudience::Tool => vec![
                "Use `background_shell_list_services {\"status\":\"ready\"}` to inspect reusable service metadata.".to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` to inspect endpoint and recipe details for the ready service."
                    ),
                    None => "Use `background_shell_attach {\"jobId\":\"<jobId|alias|@capability>\"}` to inspect endpoint and recipe details for the service you choose.".to_string(),
                },
                match provider_ref.as_deref() {
                    Some(job_ref) => match recipe_name.as_deref() {
                        Some(recipe) => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` or `background_shell_invoke_recipe {{\"jobId\":\"{job_ref}\",\"recipe\":\"{recipe}\"}}` to reuse the ready service directly."
                        ),
                        None => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{job_ref}\"}}` to inspect endpoint and recipe details for the ready service."
                        ),
                    },
                    None => "Use `background_shell_invoke_recipe {\"jobId\":\"<jobId|alias|@capability>\",\"recipe\":\"...\"}` to reuse the ready service directly.".to_string(),
                },
            ],
        };
    }
    if booting_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Booting);
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps services booting` to inspect readiness state and startup metadata."
                    .to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Run `:ps wait {job_ref} [timeoutMs]` for the booting service you need."
                    ),
                    None => "Run `:ps wait <jobId|alias|@capability|n> [timeoutMs]` for the booting service you need."
                        .to_string(),
                },
                "Run `:ps capabilities booting` to keep the capability view focused.".to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `background_shell_list_services {\"status\":\"booting\"}` to inspect readiness state and startup metadata.".to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Use `background_shell_wait_ready {{\"jobId\":\"{job_ref}\",\"timeoutMs\":5000}}` for the booting service you need."
                    ),
                    None => "Use `background_shell_wait_ready {\"jobId\":\"<jobId|alias|@capability>\",\"timeoutMs\":5000}` for the booting service you need.".to_string(),
                },
                "Use `background_shell_list_capabilities {\"status\":\"booting\"}` to keep the capability view focused.".to_string(),
            ],
        };
    }
    if untracked_services > 0 {
        let provider_ref =
            unique_service_ref_by_readiness(state, BackgroundShellServiceReadiness::Untracked);
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps services untracked` to inspect reusable services that still lack readiness or attachment contract metadata."
                    .to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Run `:ps contract {job_ref} <json-object>` to add fields such as `readyPattern`, `protocol`, `endpoint`, `attachHint`, or `recipes`."
                    ),
                    None => "Run `:ps contract <jobId|alias|@capability|n> <json-object>` to add fields such as `readyPattern`, `protocol`, `endpoint`, `attachHint`, or `recipes`."
                        .to_string(),
                },
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Run `:ps relabel {job_ref} <label|none>` if the service also needs a clearer operator-facing identity."
                    ),
                    None => "Run `:ps relabel <jobId|alias|@capability|n> <label|none>` if the service also needs a clearer operator-facing identity."
                        .to_string(),
                },
            ],
            ActionAudience::Tool => vec![
                "Use `background_shell_list_services {\"status\":\"untracked\"}` to inspect reusable services that still lack readiness or attachment contract metadata."
                    .to_string(),
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` to add reusable contract metadata in place."
                    ),
                    None => "Use `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}` to add reusable contract metadata in place."
                        .to_string(),
                },
                match provider_ref.as_deref() {
                    Some(job_ref) => format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{job_ref}\",\"label\":\"service-label\"}}` if the service also needs a clearer operator-facing identity."
                    ),
                    None => "Use `background_shell_update_service {\"jobId\":\"<jobId|alias|@capability>\",\"label\":\"service-label\"}` if the service also needs a clearer operator-facing identity."
                        .to_string(),
                },
            ],
        };
    }
    if sidecar_agents + shell_sidecars > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps agents` to inspect sidecar agent progress.".to_string(),
                "Run `:ps shells` to inspect non-blocking shell jobs.".to_string(),
                "Continue foreground work until one of those results becomes relevant."
                    .to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"agents\"}` to inspect sidecar agent progress.".to_string(),
                "Use `orchestration_list_workers {\"filter\":\"shells\"}` to inspect non-blocking shell jobs.".to_string(),
                "Continue foreground work until one of those results becomes relevant."
                    .to_string(),
            ],
        };
    }
    if terminals > 0 {
        return match audience {
            ActionAudience::Operator => vec![
                "Run `:ps terminals` to inspect server-observed background terminals."
                    .to_string(),
                "Run `:clean terminals` to close them if they are no longer needed."
                    .to_string(),
            ],
            ActionAudience::Tool => vec![
                "Use `orchestration_list_workers {\"filter\":\"terminals\"}` to inspect server-observed background terminals.".to_string(),
                "Terminal cleanup is operator-only; use `:clean terminals` from the wrapper when they are no longer needed.".to_string(),
            ],
        };
    }

    Vec::new()
}

fn action_lines_for_capability(
    state: &AppState,
    audience: ActionAudience,
    capability: &str,
) -> Result<Vec<String>, String> {
    if let Some(issue) = state
        .orchestration
        .background_shells
        .blocking_capability_dependency_issues()
        .into_iter()
        .find(|issue| issue.capability == capability)
    {
        return Ok(match (issue.status, audience) {
            (BackgroundShellCapabilityDependencyState::Missing, ActionAudience::Operator) => {
                let blocker_ref = first_blocking_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Run `:ps capabilities @{capability}` to inspect the missing provider map."
                    ),
                    format!(
                        "Run `:ps provide {service_ref} @{capability}` to retarget an existing running service, or start a new provider for that role."
                    ),
                    format!(
                        "Run `:ps depend {blocker_ref} <@capability...|none>` to retarget the blocked shell if it should wait on a different reusable role."
                    ),
                    format!(
                        "Run `:ps dependencies missing @{capability}` to inspect the blocked dependency edges."
                    ),
                    format!(
                        "If the blocked shell is no longer needed, run `:clean blockers @{capability}`."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Missing, ActionAudience::Tool) => {
                let blocker_ref = first_blocking_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the missing provider map."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget an existing running service, or start a new provider for that capability."
                    ),
                    format!(
                        "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` to retarget the blocked shell if it should depend on a different reusable role."
                    ),
                    format!(
                        "Use `orchestration_list_dependencies {{\"filter\":\"missing\",\"capability\":\"@{capability}\"}}` to inspect the blocked dependency edges."
                    ),
                    format!(
                        "Use `background_shell_clean {{\"scope\":\"blockers\",\"capability\":\"@{capability}\"}}` to abandon the blocking prerequisite shells if they are no longer needed."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Ambiguous, ActionAudience::Operator) => {
                let blocker_ref = first_blocking_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Run `:ps capabilities @{capability}` to inspect the ambiguous provider set."
                    ),
                    format!(
                        "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{capability} on one running provider before falling back to cleanup."
                    ),
                    format!(
                        "Run `:ps depend {blocker_ref} <@capability...|none>` if the blocked shell should be retargeted to a different dependency role."
                    ),
                    format!(
                        "Run `:clean services @{capability}` to clear the conflicting reusable role in one step."
                    ),
                    format!("Run `:ps services @{capability}` to inspect the remaining providers."),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Ambiguous, ActionAudience::Tool) => {
                let blocker_ref = first_blocking_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect the ambiguous provider set."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role before falling back to cleanup."
                    ),
                    format!(
                        "Use `background_shell_update_dependencies {{\"jobId\":\"{blocker_ref}\",\"dependsOnCapabilities\":[\"@other.role\"]}}` if the blocked shell should be retargeted to a different dependency role."
                    ),
                    format!(
                        "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the conflicting reusable role in one step."
                    ),
                    format!(
                        "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to inspect the remaining providers."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Booting, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Run `:ps services booting @{capability}` to inspect the booting provider state."
                    ),
                    format!(
                        "Run `:ps wait {provider_ref} 5000` to wait on the capability provider."
                    ),
                    format!(
                        "Run `:ps dependencies booting @{capability}` to keep the dependency view focused."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Booting, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect the booting provider state."
                    ),
                    format!(
                        "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` to wait on the capability provider."
                    ),
                    format!(
                        "Use `orchestration_list_dependencies {{\"filter\":\"booting\",\"capability\":\"@{capability}\"}}` to keep the dependency view focused."
                    ),
                ]
            }
            (BackgroundShellCapabilityDependencyState::Satisfied, _) => vec![],
        });
    }

    Ok(
        match (
            state
                .orchestration
                .background_shells
                .service_capability_issue_for_ref(capability)?,
            audience,
        ) {
            (BackgroundShellCapabilityIssueClass::Missing, ActionAudience::Operator) => {
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Run `:ps capabilities @{capability}` to confirm there is no running provider."
                    ),
                    format!(
                        "Run `:ps provide {service_ref} @{capability}` to retarget an existing running service, or start a new provider for that role."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Missing, ActionAudience::Tool) => {
                let service_ref = unique_running_service_ref(state)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to confirm there is no running provider."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{service_ref}\",\"capabilities\":[\"@{capability}\"]}}` to retarget an existing running service, or start a new provider for that capability."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Ambiguous, ActionAudience::Operator) => vec![
                format!("Run `:ps capabilities @{capability}` to inspect providers and consumers."),
                {
                    let provider_ref = first_provider_ref_for_capability(state, capability)
                        .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                    format!(
                        "Run `:ps provide {provider_ref} <@other.role|none>` to remove or replace @{capability} on one running provider before falling back to cleanup."
                    )
                },
                format!(
                    "Run `:clean services @{capability}` to clear the ambiguous reusable role."
                ),
                format!("Run `:ps services @{capability}` to verify the surviving providers."),
            ],
            (BackgroundShellCapabilityIssueClass::Ambiguous, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| "<jobId|alias|n>".to_string());
                vec![
                    format!(
                        "Use `background_shell_inspect_capability {{\"capability\":\"@{capability}\"}}` to inspect providers and consumers."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":[\"@other.role\"]}}` or `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"capabilities\":null}}` to remove or replace the conflicting reusable role before falling back to cleanup."
                    ),
                    format!(
                        "Use `background_shell_clean {{\"scope\":\"services\",\"capability\":\"@{capability}\"}}` to clear the ambiguous reusable role."
                    ),
                    format!(
                        "Use `background_shell_list_services {{\"capability\":\"@{capability}\"}}` to verify the surviving providers."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Booting, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Run `:ps services booting @{capability}` to inspect provider readiness."
                    ),
                    format!("Run `:ps wait {provider_ref} 5000` for the booting service you need."),
                    "Run `:ps capabilities booting` to keep the capability view focused."
                        .to_string(),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Booting, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                format!(
                    "Use `background_shell_list_services {{\"status\":\"booting\",\"capability\":\"@{capability}\"}}` to inspect provider readiness."
                ),
                format!(
                    "Use `background_shell_wait_ready {{\"jobId\":\"{provider_ref}\",\"timeoutMs\":5000}}` for the booting service you need."
                ),
                "Use `background_shell_list_capabilities {\"status\":\"booting\"}` to keep the capability view focused.".to_string(),
            ]
            }
            (BackgroundShellCapabilityIssueClass::Untracked, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Run `:ps services untracked @{capability}` to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Run `:ps contract {provider_ref} <json-object>` to add `readyPattern`, `protocol`, `endpoint`, or recipes in place for @{capability}."
                    ),
                    format!(
                        "Run `:ps relabel {provider_ref} <label|none>` if the reusable service needs a clearer operator label."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Untracked, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                vec![
                    format!(
                        "Use `background_shell_list_services {{\"status\":\"untracked\",\"capability\":\"@{capability}\"}}` to inspect the provider missing readiness or attachment metadata."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"readyPattern\":\"READY\",\"protocol\":\"http\",\"endpoint\":\"http://127.0.0.1:3000\"}}` to add a live readiness or attachment contract for @{capability}."
                    ),
                    format!(
                        "Use `background_shell_update_service {{\"jobId\":\"{provider_ref}\",\"label\":\"service-label\"}}` if the reusable service needs a clearer operator label."
                    ),
                ]
            }
            (BackgroundShellCapabilityIssueClass::Healthy, ActionAudience::Operator) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe_name = first_recipe_name_for_capability(state, capability);
                vec![
                    format!(
                        "Run `:ps attach {provider_ref}` to inspect endpoint and recipe details."
                    ),
                    match recipe_name.as_deref() {
                        Some(recipe) => format!(
                            "Run `:ps attach {provider_ref}` or `:ps run {provider_ref} {recipe} [json-args]` to reuse the ready service directly."
                        ),
                        None => format!(
                            "Run `:ps attach {provider_ref}` to inspect endpoint and recipe details."
                        ),
                    },
                ]
            }
            (BackgroundShellCapabilityIssueClass::Healthy, ActionAudience::Tool) => {
                let provider_ref = first_provider_ref_for_capability(state, capability)
                    .unwrap_or_else(|| format!("@{capability}"));
                let recipe_name = first_recipe_name_for_capability(state, capability);
                vec![
                    format!(
                        "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` to inspect endpoint and recipe details."
                    ),
                    match recipe_name.as_deref() {
                        Some(recipe) => format!(
                            "Use `background_shell_attach {{\"jobId\":\"{provider_ref}\"}}` or `background_shell_invoke_recipe {{\"jobId\":\"{provider_ref}\",\"recipe\":\"{recipe}\"}}` to reuse the ready service directly."
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
