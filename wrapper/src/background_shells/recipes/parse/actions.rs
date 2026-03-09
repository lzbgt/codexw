use super::super::super::BackgroundShellInteractionAction;

pub(super) fn parse_background_shell_interaction_action(
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
