use crate::background_shells::BackgroundShellInteractionRecipe;
use crate::background_shells::BackgroundShellManager;

use super::super::super::super::helpers::normalize_service_label_update;
use super::super::super::super::helpers::parse_service_recipe_updates;

pub(super) fn parse_service_recipes_for_tool(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<Vec<BackgroundShellInteractionRecipe>>, String> {
    if object.contains_key("recipes") {
        Ok(Some(parse_service_recipe_updates(
            object.get("recipes"),
            "background_shell_update_service",
        )?))
    } else {
        Ok(None)
    }
}

pub(super) fn require_service_update_fields(
    has_capabilities: bool,
    has_label: bool,
    has_protocol: bool,
    has_endpoint: bool,
    has_attach_hint: bool,
    has_ready_pattern: bool,
    has_recipes: bool,
) -> Result<(), String> {
    if has_capabilities
        || has_label
        || has_protocol
        || has_endpoint
        || has_attach_hint
        || has_ready_pattern
        || has_recipes
    {
        Ok(())
    } else {
        Err(
            "background_shell_update_service requires at least one mutable field such as `capabilities`, `label`, `protocol`, `endpoint`, `attachHint`, `readyPattern`, or `recipes`"
                .to_string(),
        )
    }
}

pub(super) fn apply_service_contract_updates(
    manager: &BackgroundShellManager,
    resolved_job_id: &str,
    protocol: Option<Option<String>>,
    endpoint: Option<Option<String>>,
    attach_hint: Option<Option<String>>,
    ready_pattern: Option<Option<String>>,
    interaction_recipes: Option<Vec<BackgroundShellInteractionRecipe>>,
) -> Result<
    (
        Option<Option<String>>,
        Option<Option<String>>,
        Option<Option<String>>,
        Option<Option<String>>,
        Option<usize>,
    ),
    String,
> {
    let normalized_protocol = protocol
        .clone()
        .map(normalize_service_label_update)
        .transpose()?;
    let normalized_endpoint = endpoint
        .clone()
        .map(normalize_service_label_update)
        .transpose()?;
    let normalized_attach_hint = attach_hint
        .clone()
        .map(normalize_service_label_update)
        .transpose()?;
    let normalized_ready_pattern = ready_pattern
        .clone()
        .map(normalize_service_label_update)
        .transpose()?;
    let recipe_count = interaction_recipes.as_ref().map(Vec::len);

    if protocol.is_some()
        || endpoint.is_some()
        || attach_hint.is_some()
        || ready_pattern.is_some()
        || interaction_recipes.is_some()
    {
        manager.set_running_service_contract(
            resolved_job_id,
            protocol,
            endpoint,
            attach_hint,
            ready_pattern,
            interaction_recipes,
        )?;
    }

    Ok((
        normalized_protocol,
        normalized_endpoint,
        normalized_attach_hint,
        normalized_ready_pattern,
        recipe_count,
    ))
}
