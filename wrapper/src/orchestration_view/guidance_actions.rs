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
#[path = "guidance_actions/guidance.rs"]
mod guidance;

use actions::action_lines;
use actions::action_lines_for_capability;
use guidance::guidance_lines;
use guidance::guidance_lines_for_capability;
use guidance::guidance_lines_for_tool;
use guidance::guidance_lines_for_tool_capability;

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

fn normalize_capability_ref(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().trim_start_matches('@');
    if normalized.is_empty() {
        return Err("capability selector must be a non-empty capability name".to_string());
    }
    Ok(normalized.to_string())
}
