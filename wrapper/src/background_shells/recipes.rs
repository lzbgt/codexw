use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpStream;
use std::time::Duration;

use url::Url;

use super::BackgroundShellInteractionAction;
use super::BackgroundShellInteractionParameter;
use super::BackgroundShellInteractionRecipe;
use super::parse_background_shell_optional_string;

pub(super) fn parse_background_shell_interaction_recipes(
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

pub(super) fn parse_recipe_arguments_map(
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

pub(super) fn interaction_action_summary(action: &BackgroundShellInteractionAction) -> String {
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

pub(super) fn invoke_http_recipe(
    endpoint: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
    headers: &[(String, String)],
    expected_status: Option<u16>,
) -> Result<String, String> {
    let base = Url::parse(endpoint)
        .map_err(|err| format!("invalid background shell service endpoint `{endpoint}`: {err}"))?;
    if base.scheme() != "http" {
        return Err(format!(
            "background shell service endpoint `{endpoint}` uses unsupported scheme `{}`; only plain http:// endpoints are currently invokable",
            base.scheme()
        ));
    }
    let request_url = base.join(path).map_err(|err| {
        format!("failed to resolve recipe path `{path}` against endpoint `{endpoint}`: {err}")
    })?;
    let host = request_url
        .host_str()
        .ok_or_else(|| format!("background shell service endpoint `{endpoint}` has no host"))?;
    let port = request_url
        .port_or_known_default()
        .ok_or_else(|| format!("background shell service endpoint `{endpoint}` has no port"))?;
    let request_path = match request_url.query() {
        Some(query) => format!("{}?{query}", request_url.path()),
        None => request_url.path().to_string(),
    };
    let host_header = match request_url.port() {
        Some(port)
            if (request_url.scheme() == "http" && port != 80)
                || (request_url.scheme() == "https" && port != 443) =>
        {
            format!("{host}:{port}")
        }
        _ => host.to_string(),
    };
    let payload = body.unwrap_or_default();
    let mut request =
        format!("{method} {request_path} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    if body.is_some() {
        request.push_str(&format!("Content-Length: {}\r\n", payload.len()));
        if !headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("Content-Type"))
        {
            request.push_str("Content-Type: text/plain; charset=utf-8\r\n");
        }
    }
    request.push_str("\r\n");
    if body.is_some() {
        request.push_str(payload);
    }

    let mut stream = TcpStream::connect((host, port))
        .map_err(|err| format!("failed to connect to {host}:{port}: {err}"))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("failed to write request to {host}:{port}: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("failed to flush request to {host}:{port}: {err}"))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| format!("failed to read response from {host}:{port}: {err}"))?;
    let response = parse_http_response(&String::from_utf8_lossy(&response))?;
    if let Some(expected_status) = expected_status
        && response.status_code != expected_status
    {
        return Err(format!(
            "http recipe expected status {expected_status} but received {}.\nResponse:\n{}",
            response.status_code,
            format_http_response(&response)
        ));
    }
    Ok(format_http_response(&response))
}

pub(super) fn invoke_tcp_recipe(
    endpoint: &str,
    payload: Option<&str>,
    append_newline: bool,
    expect_substring: Option<&str>,
    read_timeout_ms: Option<u64>,
) -> Result<String, String> {
    let (host, port) = parse_tcp_endpoint(endpoint)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|err| format!("failed to connect to {host}:{port}: {err}"))?;
    let timeout = Duration::from_millis(read_timeout_ms.unwrap_or(500));
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|err| format!("failed to set read timeout for {host}:{port}: {err}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|err| format!("failed to set write timeout for {host}:{port}: {err}"))?;

    let mut payload_line = None;
    if let Some(payload) = payload {
        let mut outbound = payload.as_bytes().to_vec();
        if append_newline {
            outbound.push(b'\n');
        }
        stream
            .write_all(&outbound)
            .map_err(|err| format!("failed to write tcp payload to {host}:{port}: {err}"))?;
        stream
            .flush()
            .map_err(|err| format!("failed to flush tcp payload to {host}:{port}: {err}"))?;
        payload_line = Some(String::from_utf8_lossy(&outbound).into_owned());
    }
    let _ = stream.shutdown(Shutdown::Write);

    let mut response = Vec::new();
    let mut buf = [0_u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(bytes) => response.extend_from_slice(&buf[..bytes]),
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(err) => {
                return Err(format!(
                    "failed to read tcp response from {host}:{port}: {err}"
                ));
            }
        }
    }

    let response_text = String::from_utf8_lossy(&response).into_owned();
    if let Some(expect_substring) = expect_substring
        && !response_text.contains(expect_substring)
    {
        return Err(format!(
            "tcp recipe expected substring `{expect_substring}` but it was not observed.\nResponse:\n{}",
            format_tcp_response(host.as_str(), port, payload_line.as_deref(), &response_text)
        ));
    }

    Ok(format_tcp_response(
        host.as_str(),
        port,
        payload_line.as_deref(),
        &response_text,
    ))
}

pub(super) fn invoke_redis_recipe(
    endpoint: &str,
    command: &[String],
    expect_substring: Option<&str>,
    read_timeout_ms: Option<u64>,
) -> Result<String, String> {
    let (host, port) = parse_tcp_endpoint(endpoint)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|err| format!("failed to connect to {host}:{port}: {err}"))?;
    let timeout = Duration::from_millis(read_timeout_ms.unwrap_or(500));
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|err| format!("failed to set read timeout for {host}:{port}: {err}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|err| format!("failed to set write timeout for {host}:{port}: {err}"))?;

    let request = encode_redis_command(command);
    stream
        .write_all(&request)
        .map_err(|err| format!("failed to write redis command to {host}:{port}: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("failed to flush redis command to {host}:{port}: {err}"))?;
    let _ = stream.shutdown(Shutdown::Write);

    let mut response = Vec::new();
    let mut buf = [0_u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(bytes) => response.extend_from_slice(&buf[..bytes]),
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(err) => {
                return Err(format!(
                    "failed to read redis response from {host}:{port}: {err}"
                ));
            }
        }
    }
    let value = parse_redis_response(&response)?;
    let rendered = format_redis_response(host.as_str(), port, command, &value);
    if let Some(expect_substring) = expect_substring
        && !rendered.contains(expect_substring)
    {
        return Err(format!(
            "redis recipe expected substring `{expect_substring}` but it was not observed.\nResponse:\n{rendered}"
        ));
    }
    Ok(rendered)
}

fn parse_tcp_endpoint(endpoint: &str) -> Result<(String, u16), String> {
    if endpoint.contains("://") {
        let url = Url::parse(endpoint)
            .map_err(|err| format!("invalid tcp endpoint `{endpoint}`: {err}"))?;
        if url.scheme() != "tcp" {
            return Err(format!(
                "background shell service endpoint `{endpoint}` uses unsupported scheme `{}` for tcp recipes; use tcp://host:port",
                url.scheme()
            ));
        }
        let host = url
            .host_str()
            .ok_or_else(|| format!("tcp endpoint `{endpoint}` has no host"))?
            .to_string();
        let port = url
            .port()
            .ok_or_else(|| format!("tcp endpoint `{endpoint}` has no explicit port"))?;
        return Ok((host, port));
    }
    let (host, port) = endpoint.rsplit_once(':').ok_or_else(|| {
        format!("tcp endpoint `{endpoint}` must be `host:port` or `tcp://host:port`")
    })?;
    let port = port
        .parse::<u16>()
        .map_err(|err| format!("invalid tcp port in endpoint `{endpoint}`: {err}"))?;
    if host.trim().is_empty() {
        return Err(format!("tcp endpoint `{endpoint}` has an empty host"));
    }
    Ok((host.to_string(), port))
}

fn encode_redis_command(command: &[String]) -> Vec<u8> {
    let mut encoded = format!("*{}\r\n", command.len()).into_bytes();
    for argument in command {
        encoded.extend_from_slice(format!("${}\r\n", argument.len()).as_bytes());
        encoded.extend_from_slice(argument.as_bytes());
        encoded.extend_from_slice(b"\r\n");
    }
    encoded
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RedisRespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<RedisRespValue>>),
}

fn parse_redis_response(bytes: &[u8]) -> Result<RedisRespValue, String> {
    let mut cursor = 0;
    let value = parse_redis_value(bytes, &mut cursor)?;
    Ok(value)
}

fn parse_redis_value(bytes: &[u8], cursor: &mut usize) -> Result<RedisRespValue, String> {
    let marker = *bytes
        .get(*cursor)
        .ok_or_else(|| "redis response was empty".to_string())?;
    *cursor += 1;
    match marker {
        b'+' => Ok(RedisRespValue::SimpleString(read_redis_line(
            bytes, cursor,
        )?)),
        b'-' => Ok(RedisRespValue::Error(read_redis_line(bytes, cursor)?)),
        b':' => {
            let line = read_redis_line(bytes, cursor)?;
            let value = line
                .parse::<i64>()
                .map_err(|err| format!("invalid redis integer response `{line}`: {err}"))?;
            Ok(RedisRespValue::Integer(value))
        }
        b'$' => {
            let line = read_redis_line(bytes, cursor)?;
            let len = line
                .parse::<i64>()
                .map_err(|err| format!("invalid redis bulk length `{line}`: {err}"))?;
            if len == -1 {
                return Ok(RedisRespValue::BulkString(None));
            }
            let len =
                usize::try_from(len).map_err(|_| format!("invalid redis bulk length `{line}`"))?;
            let start = *cursor;
            let end = start + len;
            let payload = bytes
                .get(start..end)
                .ok_or_else(|| "redis bulk string was truncated".to_string())?
                .to_vec();
            *cursor = end;
            consume_redis_crlf(bytes, cursor)?;
            Ok(RedisRespValue::BulkString(Some(payload)))
        }
        b'*' => {
            let line = read_redis_line(bytes, cursor)?;
            let len = line
                .parse::<i64>()
                .map_err(|err| format!("invalid redis array length `{line}`: {err}"))?;
            if len == -1 {
                return Ok(RedisRespValue::Array(None));
            }
            let len =
                usize::try_from(len).map_err(|_| format!("invalid redis array length `{line}`"))?;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(parse_redis_value(bytes, cursor)?);
            }
            Ok(RedisRespValue::Array(Some(items)))
        }
        other => Err(format!(
            "unsupported redis response type byte `{}`",
            other as char
        )),
    }
}

fn read_redis_line(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    let start = *cursor;
    while *cursor + 1 < bytes.len() {
        if bytes[*cursor] == b'\r' && bytes[*cursor + 1] == b'\n' {
            let line = String::from_utf8_lossy(&bytes[start..*cursor]).into_owned();
            *cursor += 2;
            return Ok(line);
        }
        *cursor += 1;
    }
    Err("redis response line was truncated".to_string())
}

fn consume_redis_crlf(bytes: &[u8], cursor: &mut usize) -> Result<(), String> {
    if bytes.get(*cursor) == Some(&b'\r') && bytes.get(*cursor + 1) == Some(&b'\n') {
        *cursor += 2;
        Ok(())
    } else {
        Err("redis bulk string terminator was missing".to_string())
    }
}

fn format_redis_response(
    host: &str,
    port: u16,
    command: &[String],
    value: &RedisRespValue,
) -> String {
    let mut lines = vec![
        format!("Address: {host}:{port}"),
        format!("Command: {}", command.join(" ")),
    ];
    lines.extend(render_redis_value(value, 0));
    lines.join("\n")
}

fn render_redis_value(value: &RedisRespValue, depth: usize) -> Vec<String> {
    let indent = "  ".repeat(depth);
    match value {
        RedisRespValue::SimpleString(text) => vec![
            format!("{indent}Type: simple-string"),
            format!("{indent}Value: {text}"),
        ],
        RedisRespValue::Error(text) => vec![
            format!("{indent}Type: error"),
            format!("{indent}Value: {text}"),
        ],
        RedisRespValue::Integer(value) => vec![
            format!("{indent}Type: integer"),
            format!("{indent}Value: {value}"),
        ],
        RedisRespValue::BulkString(None) => vec![
            format!("{indent}Type: bulk-string"),
            format!("{indent}Value: (nil)"),
        ],
        RedisRespValue::BulkString(Some(bytes)) => vec![
            format!("{indent}Type: bulk-string"),
            format!("{indent}Value: {}", String::from_utf8_lossy(bytes)),
        ],
        RedisRespValue::Array(None) => vec![
            format!("{indent}Type: array"),
            format!("{indent}Value: (nil)"),
        ],
        RedisRespValue::Array(Some(items)) => {
            let mut lines = vec![
                format!("{indent}Type: array"),
                format!("{indent}Length: {}", items.len()),
            ];
            for (index, item) in items.iter().enumerate() {
                lines.push(format!("{indent}Item {index}:"));
                lines.extend(render_redis_value(item, depth + 1));
            }
            lines
        }
    }
}

fn format_tcp_response(host: &str, port: u16, payload: Option<&str>, response: &str) -> String {
    let mut lines = vec![format!("Address: {host}:{port}")];
    if let Some(payload) = payload {
        lines.push("Payload:".to_string());
        lines.extend(payload.lines().map(ToOwned::to_owned));
    }
    if response.is_empty() {
        lines.push("Body: (empty)".to_string());
    } else {
        lines.push("Body:".to_string());
        lines.extend(response.lines().map(ToOwned::to_owned));
    }
    lines.join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedHttpResponse {
    status_line: String,
    status_code: u16,
    headers: Vec<(String, String)>,
    body: String,
}

fn parse_http_response(response: &str) -> Result<ParsedHttpResponse, String> {
    let (head, body) = response
        .split_once("\r\n\r\n")
        .or_else(|| response.split_once("\n\n"))
        .ok_or_else(|| {
            "http recipe returned a malformed response without header separator".to_string()
        })?;
    let mut lines = head.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| "http recipe returned an empty response".to_string())?
        .trim_end_matches('\r')
        .to_string();
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("unable to parse HTTP status line `{status_line}`"))?
        .parse::<u16>()
        .map_err(|err| format!("unable to parse HTTP status code from `{status_line}`: {err}"))?;
    let headers = lines
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect::<Vec<_>>();
    Ok(ParsedHttpResponse {
        status_line,
        status_code,
        headers,
        body: body.to_string(),
    })
}

fn format_http_response(response: &ParsedHttpResponse) -> String {
    let mut lines = vec![
        format!("Status: {}", response.status_line),
        format!("Status code: {}", response.status_code),
    ];
    if !response.headers.is_empty() {
        lines.push("Headers:".to_string());
        for (name, value) in &response.headers {
            lines.push(format!("- {name}: {value}"));
        }
    }
    if response.body.is_empty() {
        lines.push("Body: (empty)".to_string());
    } else {
        lines.push("Body:".to_string());
        lines.extend(response.body.lines().map(ToOwned::to_owned));
    }
    lines.join("\n")
}
