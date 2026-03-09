use super::super::super::BackgroundShellInteractionParameter;
use super::super::super::BackgroundShellInteractionRecipe;
use super::super::super::parse_background_shell_optional_string;
use super::actions::parse_background_shell_interaction_action;

pub(crate) fn parse_background_shell_interaction_recipes(
    value: Option<&serde_json::Value>,
) -> Result<Vec<BackgroundShellInteractionRecipe>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let recipes = value
        .as_array()
        .ok_or_else(|| "background_shell_start `recipes` must be an array".to_string())?;
    let mut parsed = Vec::with_capacity(recipes.len());
    for (index, recipe) in recipes.iter().enumerate() {
        let object = recipe.as_object().ok_or_else(|| {
            format!("background_shell_start `recipes[{index}]` must be an object")
        })?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!("background_shell_start `recipes[{index}].name` must be a non-empty string")
            })?
            .to_string();
        let description = parse_background_shell_optional_string(
            object.get("description"),
            &format!("recipes[{index}].description"),
        )?;
        let example = parse_background_shell_optional_string(
            object.get("example"),
            &format!("recipes[{index}].example"),
        )?;
        let parameters =
            parse_background_shell_interaction_parameters(object.get("parameters"), index)?;
        let action = parse_background_shell_interaction_action(object.get("action"), index)?;
        parsed.push(BackgroundShellInteractionRecipe {
            name,
            description,
            example,
            parameters,
            action,
        });
    }
    Ok(parsed)
}

fn parse_background_shell_interaction_parameters(
    value: Option<&serde_json::Value>,
    index: usize,
) -> Result<Vec<BackgroundShellInteractionParameter>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let parameters = value.as_array().ok_or_else(|| {
        format!("background_shell_start `recipes[{index}].parameters` must be an array")
    })?;
    let mut parsed = Vec::with_capacity(parameters.len());
    for (param_index, parameter) in parameters.iter().enumerate() {
        let object = parameter.as_object().ok_or_else(|| {
            format!(
                "background_shell_start `recipes[{index}].parameters[{param_index}]` must be an object"
            )
        })?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "background_shell_start `recipes[{index}].parameters[{param_index}].name` must be a non-empty string"
                )
            })?
            .to_string();
        let description = parse_background_shell_optional_string(
            object.get("description"),
            &format!("recipes[{index}].parameters[{param_index}].description"),
        )?;
        let default = parse_background_shell_optional_string(
            object.get("default"),
            &format!("recipes[{index}].parameters[{param_index}].default"),
        )?;
        let required = object
            .get("required")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(default.is_none());
        parsed.push(BackgroundShellInteractionParameter {
            name,
            description,
            default,
            required,
        });
    }
    Ok(parsed)
}
