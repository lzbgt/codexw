use std::collections::HashMap;

use super::super::super::BackgroundShellInteractionAction;
use super::super::super::BackgroundShellInteractionParameter;

pub(crate) fn parse_recipe_arguments_map(
    value: Option<&serde_json::Value>,
    field_name: &str,
) -> Result<HashMap<String, String>, String> {
    let Some(value) = value else {
        return Ok(HashMap::new());
    };
    let object = value
        .as_object()
        .ok_or_else(|| format!("{field_name} `args` must be an object"))?;
    let mut args = HashMap::with_capacity(object.len());
    for (key, value) in object {
        let rendered = match value {
            serde_json::Value::String(text) => text.clone(),
            serde_json::Value::Bool(flag) => flag.to_string(),
            serde_json::Value::Number(number) => number.to_string(),
            _ => {
                return Err(format!(
                    "{field_name} `args.{key}` must be a string, number, or boolean"
                ));
            }
        };
        args.insert(key.clone(), rendered);
    }
    Ok(args)
}

pub(crate) fn resolve_recipe_arguments(
    parameters: &[BackgroundShellInteractionParameter],
    provided: &HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    let mut resolved = HashMap::with_capacity(parameters.len());
    for parameter in parameters {
        if let Some(value) = provided.get(&parameter.name) {
            resolved.insert(parameter.name.clone(), value.clone());
        } else if let Some(default) = parameter.default.as_deref() {
            resolved.insert(parameter.name.clone(), default.to_string());
        } else if parameter.required {
            return Err(format!(
                "recipe parameter `{}` is required but was not provided",
                parameter.name
            ));
        }
    }
    for key in provided.keys() {
        if !parameters.iter().any(|parameter| parameter.name == *key) {
            return Err(format!("unknown recipe argument `{key}`"));
        }
    }
    Ok(resolved)
}

pub(crate) fn apply_recipe_arguments_to_action(
    action: BackgroundShellInteractionAction,
    args: &HashMap<String, String>,
) -> Result<BackgroundShellInteractionAction, String> {
    Ok(match action {
        BackgroundShellInteractionAction::Informational => {
            BackgroundShellInteractionAction::Informational
        }
        BackgroundShellInteractionAction::Stdin {
            text,
            append_newline,
        } => BackgroundShellInteractionAction::Stdin {
            text: substitute_recipe_arguments(&text, args)?,
            append_newline,
        },
        BackgroundShellInteractionAction::Http {
            method,
            path,
            body,
            headers,
            expected_status,
        } => BackgroundShellInteractionAction::Http {
            method: substitute_recipe_arguments(&method, args)?,
            path: substitute_recipe_arguments(&path, args)?,
            body: body
                .as_deref()
                .map(|body| substitute_recipe_arguments(body, args))
                .transpose()?,
            headers: headers
                .into_iter()
                .map(|(name, value)| {
                    Ok((
                        substitute_recipe_arguments(&name, args)?,
                        substitute_recipe_arguments(&value, args)?,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?,
            expected_status,
        },
        BackgroundShellInteractionAction::Tcp {
            payload,
            append_newline,
            expect_substring,
            read_timeout_ms,
        } => BackgroundShellInteractionAction::Tcp {
            payload: payload
                .as_deref()
                .map(|payload| substitute_recipe_arguments(payload, args))
                .transpose()?,
            append_newline,
            expect_substring: expect_substring
                .as_deref()
                .map(|text| substitute_recipe_arguments(text, args))
                .transpose()?,
            read_timeout_ms,
        },
        BackgroundShellInteractionAction::Redis {
            command,
            expect_substring,
            read_timeout_ms,
        } => BackgroundShellInteractionAction::Redis {
            command: command
                .into_iter()
                .map(|item| substitute_recipe_arguments(&item, args))
                .collect::<Result<Vec<_>, String>>()?,
            expect_substring: expect_substring
                .as_deref()
                .map(|text| substitute_recipe_arguments(text, args))
                .transpose()?,
            read_timeout_ms,
        },
    })
}

fn substitute_recipe_arguments(
    template: &str,
    args: &HashMap<String, String>,
) -> Result<String, String> {
    let mut rendered = String::with_capacity(template.len());
    let mut cursor = 0;
    while let Some(start_rel) = template[cursor..].find("{{") {
        let start = cursor + start_rel;
        rendered.push_str(&template[cursor..start]);
        let rest = &template[start + 2..];
        let Some(end_rel) = rest.find("}}") else {
            return Err(format!("unterminated recipe placeholder in `{template}`"));
        };
        let end = start + 2 + end_rel;
        let key = template[start + 2..end].trim();
        let value = args
            .get(key)
            .ok_or_else(|| format!("recipe placeholder `{{{{{key}}}}}` was not provided"))?;
        rendered.push_str(value);
        cursor = end + 2;
    }
    rendered.push_str(&template[cursor..]);
    Ok(rendered)
}
