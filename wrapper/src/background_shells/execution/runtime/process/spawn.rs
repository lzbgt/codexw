use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use super::super::super::super::BackgroundShellIntent;
pub(crate) fn resolve_background_cwd(
    raw_cwd: Option<&str>,
    resolved_cwd: &str,
) -> Result<PathBuf, String> {
    let base = PathBuf::from(resolved_cwd);
    let cwd = match raw_cwd {
        Some(raw) => {
            let path = PathBuf::from(raw);
            if path.is_absolute() {
                path
            } else {
                base.join(path)
            }
        }
        None => base,
    };
    if !cwd.exists() {
        return Err(format!(
            "background shell cwd `{}` does not exist",
            cwd.display()
        ));
    }
    if !cwd.is_dir() {
        return Err(format!(
            "background shell cwd `{}` is not a directory",
            cwd.display()
        ));
    }
    Ok(cwd)
}

pub(crate) fn parse_background_shell_intent(
    value: Option<&serde_json::Value>,
) -> Result<BackgroundShellIntent, String> {
    let Some(raw) = value else {
        return Ok(BackgroundShellIntent::Observation);
    };
    let raw = raw
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "background_shell_start `intent` must be one of `prerequisite`, `observation`, or `service`".to_string()
        })?;
    BackgroundShellIntent::from_str(raw).ok_or_else(|| {
        "background_shell_start `intent` must be one of `prerequisite`, `observation`, or `service`".to_string()
    })
}

pub(crate) fn parse_background_shell_label(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn parse_background_shell_optional_string(
    value: Option<&serde_json::Value>,
    field_name: &str,
) -> Result<Option<String>, String> {
    match value {
        None => Ok(None),
        Some(raw) => raw
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .map(Some)
            .ok_or_else(|| {
                format!("background_shell_start `{field_name}` must be a non-empty string")
            }),
    }
}

pub(crate) fn parse_background_shell_ready_pattern(
    value: Option<&serde_json::Value>,
) -> Result<Option<String>, String> {
    parse_background_shell_optional_string(value, "readyPattern")
}

pub(crate) fn parse_background_shell_capabilities(
    value: Option<&serde_json::Value>,
    field_name: &str,
) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| format!("background_shell_start `{field_name}` must be an array"))?;
    let mut capabilities = Vec::with_capacity(array.len());
    for (index, item) in array.iter().enumerate() {
        let raw = item.as_str().ok_or_else(|| {
            format!("background_shell_start `{field_name}[{index}]` must be a string")
        })?;
        capabilities.push(validate_service_capability(raw)?);
    }
    capabilities.sort();
    capabilities.dedup();
    Ok(capabilities)
}

pub(crate) fn parse_background_shell_timeout_ms(
    value: Option<&serde_json::Value>,
    context: &str,
) -> Result<Option<u64>, String> {
    match value {
        None => Ok(None),
        Some(raw) => raw
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{context} timeout field must be a non-negative integer")),
    }
}

pub(crate) fn validate_alias(alias: &str) -> Result<String, String> {
    let alias = alias.trim();
    if alias.is_empty() {
        return Err("background shell alias cannot be empty".to_string());
    }
    if alias
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        Ok(alias.to_string())
    } else {
        Err("background shell alias must use only letters, digits, '.', '-' or '_'".to_string())
    }
}

pub(crate) fn validate_service_capability(capability: &str) -> Result<String, String> {
    let capability = capability.trim();
    if capability.is_empty() {
        return Err("background shell capability cannot be empty".to_string());
    }
    if capability
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/'))
    {
        Ok(capability.to_string())
    } else {
        Err(
            "background shell capability must use only letters, digits, '.', '-', '_' or '/'"
                .to_string(),
        )
    }
}

pub(crate) fn spawn_shell_process(
    command: &str,
    cwd: &Path,
) -> Result<std::process::Child, String> {
    let mut shell = shell_command(command);
    shell.current_dir(cwd);
    shell.stdin(Stdio::piped());
    shell.stdout(Stdio::piped());
    shell.stderr(Stdio::piped());
    shell
        .spawn()
        .map_err(|err| format!("failed to start background shell command: {err}"))
}

#[cfg(unix)]
fn shell_command(command: &str) -> Command {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut process = Command::new(shell);
    process.arg("-lc").arg(command);
    process
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut process = Command::new("cmd");
    process.arg("/C").arg(command);
    process
}
