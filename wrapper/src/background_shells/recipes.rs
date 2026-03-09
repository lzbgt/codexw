use std::collections::HashMap;

#[path = "recipes/transports.rs"]
mod transports;

pub(crate) use self::transports::invoke_http_recipe;
pub(crate) use self::transports::invoke_redis_recipe;
pub(crate) use self::transports::invoke_tcp_recipe;
use super::BackgroundShellInteractionAction;
use super::BackgroundShellInteractionParameter;
use super::BackgroundShellInteractionRecipe;
use super::parse_background_shell_optional_string;

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

pub(super) fn resolve_recipe_arguments(
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

pub(super) fn apply_recipe_arguments_to_action(
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

pub(super) fn render_recipe_parameters(
    parameters: &[BackgroundShellInteractionParameter],
) -> String {
    parameters
        .iter()
        .map(|parameter| {
            let mut rendered = parameter.name.clone();
            if parameter.required {
                rendered.push('*');
            }
            if let Some(default) = parameter.default.as_deref() {
                rendered.push_str(&format!("={default}"));
            }
            rendered
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_background_shell_interaction_action(
    value: Option<&serde_json::Value>,
    index: usize,
) -> Result<BackgroundShellInteractionAction, String> {
    let Some(value) = value else {
        return Ok(BackgroundShellInteractionAction::Informational);
    };
    let object = value.as_object().ok_or_else(|| {
        format!("background_shell_start `recipes[{index}].action` must be an object")
    })?;
    let action_type = object
        .get("type")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "background_shell_start `recipes[{index}].action.type` must be a non-empty string"
            )
        })?;
    match action_type {
        "stdin" => {
            let text = object
                .get("text")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    format!(
                        "background_shell_start `recipes[{index}].action.text` must be a string"
                    )
                })?
                .to_string();
            let append_newline = object
                .get("appendNewline")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            Ok(BackgroundShellInteractionAction::Stdin {
                text,
                append_newline,
            })
        }
        "http" => {
            let method = object
                .get("method")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    format!(
                        "background_shell_start `recipes[{index}].action.method` must be a non-empty string"
                    )
                })?
                .to_ascii_uppercase();
            let path = object
                .get("path")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    format!(
                        "background_shell_start `recipes[{index}].action.path` must be a non-empty string"
                    )
                })?
                .to_string();
            let body = match object.get("body") {
                None | Some(serde_json::Value::Null) => None,
                Some(value) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            format!(
                                "background_shell_start `recipes[{index}].action.body` must be a string"
                            )
                        })?
                        .to_string(),
                ),
            };
            let headers = parse_background_shell_http_headers(object.get("headers"), index)?;
            let expected_status =
                parse_background_shell_expected_status(object.get("expectedStatus"), index)?;
            Ok(BackgroundShellInteractionAction::Http {
                method,
                path,
                body,
                headers,
                expected_status,
            })
        }
        "tcp" => {
            let payload = match object.get("payload") {
                None | Some(serde_json::Value::Null) => None,
                Some(value) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            format!(
                                "background_shell_start `recipes[{index}].action.payload` must be a string"
                            )
                        })?
                        .to_string(),
                ),
            };
            let append_newline = object
                .get("appendNewline")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let expect_substring = match object.get("expectSubstring") {
                None | Some(serde_json::Value::Null) => None,
                Some(value) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            format!(
                                "background_shell_start `recipes[{index}].action.expectSubstring` must be a string"
                            )
                        })?
                        .to_string(),
                ),
            };
            let read_timeout_ms = match object.get("readTimeoutMs") {
                None | Some(serde_json::Value::Null) => None,
                Some(value) => Some(value.as_u64().ok_or_else(|| {
                    format!(
                        "background_shell_start `recipes[{index}].action.readTimeoutMs` must be an integer"
                    )
                })?),
            };
            Ok(BackgroundShellInteractionAction::Tcp {
                payload,
                append_newline,
                expect_substring,
                read_timeout_ms,
            })
        }
        "redis" => {
            let command = object
                .get("command")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    format!(
                        "background_shell_start `recipes[{index}].action.command` must be an array of strings"
                    )
                })?
                .iter()
                .enumerate()
                .map(|(arg_index, value)| {
                    value.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                        format!(
                            "background_shell_start `recipes[{index}].action.command[{arg_index}]` must be a string"
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if command.is_empty() {
                return Err(format!(
                    "background_shell_start `recipes[{index}].action.command` must not be empty"
                ));
            }
            let expect_substring = match object.get("expectSubstring") {
                None | Some(serde_json::Value::Null) => None,
                Some(value) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            format!(
                                "background_shell_start `recipes[{index}].action.expectSubstring` must be a string"
                            )
                        })?
                        .to_string(),
                ),
            };
            let read_timeout_ms = match object.get("readTimeoutMs") {
                None | Some(serde_json::Value::Null) => None,
                Some(value) => Some(value.as_u64().ok_or_else(|| {
                    format!(
                        "background_shell_start `recipes[{index}].action.readTimeoutMs` must be an integer"
                    )
                })?),
            };
            Ok(BackgroundShellInteractionAction::Redis {
                command,
                expect_substring,
                read_timeout_ms,
            })
        }
        "info" | "informational" => Ok(BackgroundShellInteractionAction::Informational),
        _ => Err(format!(
            "background_shell_start `recipes[{index}].action.type` must be one of `stdin`, `http`, `tcp`, `redis`, or `informational`"
        )),
    }
}

fn parse_background_shell_http_headers(
    value: Option<&serde_json::Value>,
    index: usize,
) -> Result<Vec<(String, String)>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let object = value.as_object().ok_or_else(|| {
        format!("background_shell_start `recipes[{index}].action.headers` must be an object")
    })?;
    let mut headers = Vec::with_capacity(object.len());
    for (key, value) in object {
        if key.trim().is_empty() {
            return Err(format!(
                "background_shell_start `recipes[{index}].action.headers` keys must be non-empty"
            ));
        }
        let header_value = value.as_str().ok_or_else(|| {
            format!(
                "background_shell_start `recipes[{index}].action.headers.{key}` must be a string"
            )
        })?;
        headers.push((key.clone(), header_value.to_string()));
    }
    Ok(headers)
}

fn parse_background_shell_expected_status(
    value: Option<&serde_json::Value>,
    index: usize,
) -> Result<Option<u16>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let status = value.as_u64().ok_or_else(|| {
        format!(
            "background_shell_start `recipes[{index}].action.expectedStatus` must be an integer"
        )
    })?;
    let status = u16::try_from(status).map_err(|_| {
        format!("background_shell_start `recipes[{index}].action.expectedStatus` must fit in u16")
    })?;
    if !(100..=599).contains(&status) {
        return Err(format!(
            "background_shell_start `recipes[{index}].action.expectedStatus` must be between 100 and 599"
        ));
    }
    Ok(Some(status))
}

pub(crate) fn interaction_action_summary(action: &BackgroundShellInteractionAction) -> String {
    match action {
        BackgroundShellInteractionAction::Informational => "info".to_string(),
        BackgroundShellInteractionAction::Stdin {
            text,
            append_newline,
        } => {
            let mut summary = format!("stdin \"{}\"", summarize_recipe_text(text));
            if !append_newline {
                summary.push_str(" no-newline");
            }
            summary
        }
        BackgroundShellInteractionAction::Http {
            method,
            path,
            body,
            headers,
            expected_status,
        } => {
            let mut summary = format!("http {method} {path}");
            if !headers.is_empty() {
                summary.push_str(&format!(" headers={}", headers.len()));
            }
            if let Some(body) = body.as_deref() {
                summary.push_str(&format!(" body={}b", body.len()));
            }
            if let Some(expected_status) = expected_status {
                summary.push_str(&format!(" expect={expected_status}"));
            }
            summary
        }
        BackgroundShellInteractionAction::Tcp {
            payload,
            append_newline,
            expect_substring,
            read_timeout_ms,
        } => {
            let mut summary = "tcp".to_string();
            if let Some(payload) = payload.as_deref() {
                summary.push_str(&format!(" payload=\"{}\"", summarize_recipe_text(payload)));
                if *append_newline {
                    summary.push_str(" newline");
                }
            }
            if let Some(expect_substring) = expect_substring.as_deref() {
                summary.push_str(&format!(
                    " expect=\"{}\"",
                    summarize_recipe_text(expect_substring)
                ));
            }
            if let Some(timeout_ms) = read_timeout_ms {
                summary.push_str(&format!(" timeout={}ms", timeout_ms));
            }
            summary
        }
        BackgroundShellInteractionAction::Redis {
            command,
            expect_substring,
            read_timeout_ms,
        } => {
            let mut summary = format!("redis {}", command.join(" "));
            if let Some(expect_substring) = expect_substring.as_deref() {
                summary.push_str(&format!(
                    " expect=\"{}\"",
                    summarize_recipe_text(expect_substring)
                ));
            }
            if let Some(timeout_ms) = read_timeout_ms {
                summary.push_str(&format!(" timeout={}ms", timeout_ms));
            }
            summary
        }
    }
}

fn summarize_recipe_text(text: &str) -> String {
    const MAX_CHARS: usize = 40;
    let sanitized = text.replace('\n', "\\n");
    let mut chars = sanitized.chars();
    let summary = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{summary}...")
    } else {
        summary
    }
}
