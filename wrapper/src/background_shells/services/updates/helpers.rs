use super::super::super::BackgroundShellInteractionRecipe;
use super::super::super::parse_background_shell_interaction_recipes;
use super::super::super::validate_service_capability;

pub(super) fn normalize_service_capabilities(
    capabilities: &[String],
) -> Result<Vec<String>, String> {
    let mut normalized = capabilities
        .iter()
        .map(|capability| validate_service_capability(capability.trim_start_matches('@')))
        .collect::<Result<Vec<_>, _>>()?;
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

pub(super) fn normalize_service_label_update(
    label: Option<String>,
) -> Result<Option<String>, String> {
    match label {
        Some(label) => {
            let trimmed = label.trim();
            if trimmed.is_empty() {
                Err("service label cannot be empty".to_string())
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        None => Ok(None),
    }
}

pub(super) fn parse_service_capabilities_argument(
    value: Option<&serde_json::Value>,
    context: &str,
    field_name: &str,
) -> Result<Vec<String>, String> {
    let value = value.ok_or_else(|| format!("{context} requires `{field_name}`"))?;
    if matches!(value, serde_json::Value::Null) {
        return Ok(Vec::new());
    }
    let array = value
        .as_array()
        .ok_or_else(|| format!("{context} `{field_name}` must be an array or null"))?;
    let raw = array
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{context} `{field_name}[{index}]` must be a string"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    normalize_service_capabilities(&raw)
}

pub(super) fn parse_service_string_update_argument(
    value: Option<&serde_json::Value>,
    context: &str,
    field_name: &str,
) -> Result<Option<String>, String> {
    match value {
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(label)) => {
            normalize_service_label_update(Some(label.to_string()))
        }
        Some(_) => Err(format!("{context} `{field_name}` must be a string or null")),
        None => Err(format!("{context} requires `{field_name}`")),
    }
}

pub(super) fn parse_service_recipe_updates(
    value: Option<&serde_json::Value>,
    context: &str,
) -> Result<Vec<BackgroundShellInteractionRecipe>, String> {
    match value {
        Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(value) => parse_background_shell_interaction_recipes(Some(value))
            .map_err(|err| format!("{context}: {err}")),
        None => Err(format!("{context} requires `recipes`")),
    }
}

pub(super) fn render_service_metadata_update_summary(
    job_id: &str,
    capabilities: Option<&[String]>,
    label: Option<Option<String>>,
    protocol: Option<Option<String>>,
    endpoint: Option<Option<String>>,
    attach_hint: Option<Option<String>>,
    ready_pattern: Option<Option<String>>,
    recipe_count: Option<usize>,
) -> String {
    let mut parts = Vec::new();
    if let Some(capabilities) = capabilities {
        if capabilities.is_empty() {
            parts.push("cleared reusable capabilities".to_string());
        } else {
            parts.push(format!(
                "reusable capabilities={}",
                capabilities
                    .iter()
                    .map(|capability| format!("@{capability}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    if let Some(label) = label {
        match label {
            Some(label) => parts.push(format!("label={label}")),
            None => parts.push("cleared label".to_string()),
        }
    }
    if let Some(protocol) = protocol {
        match protocol {
            Some(protocol) => parts.push(format!("protocol={protocol}")),
            None => parts.push("cleared protocol".to_string()),
        }
    }
    if let Some(endpoint) = endpoint {
        match endpoint {
            Some(endpoint) => parts.push(format!("endpoint={endpoint}")),
            None => parts.push("cleared endpoint".to_string()),
        }
    }
    if let Some(attach_hint) = attach_hint {
        match attach_hint {
            Some(attach_hint) => parts.push(format!("attachHint={attach_hint}")),
            None => parts.push("cleared attachHint".to_string()),
        }
    }
    if let Some(ready_pattern) = ready_pattern {
        match ready_pattern {
            Some(ready_pattern) => parts.push(format!("readyPattern={ready_pattern}")),
            None => parts.push("cleared readyPattern".to_string()),
        }
    }
    if let Some(recipe_count) = recipe_count {
        if recipe_count == 0 {
            parts.push("cleared recipes".to_string());
        } else {
            parts.push(format!("recipes={recipe_count}"));
        }
    }

    if parts.is_empty() {
        format!("No service metadata changed for background shell job {job_id}.")
    } else {
        format!(
            "Updated service metadata for background shell job {job_id}: {}.",
            parts.join("; ")
        )
    }
}

pub(super) fn render_dependency_capability_update_summary(
    job_id: &str,
    capabilities: &[String],
) -> String {
    if capabilities.is_empty() {
        format!("Cleared dependency capabilities for background shell job {job_id}.")
    } else {
        format!(
            "Updated dependency capabilities for background shell job {job_id}: {}",
            capabilities
                .iter()
                .map(|capability| format!("@{capability}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
