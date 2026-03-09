use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use super::super::BackgroundShellIntent;
use super::super::BackgroundShellInteractionAction;
use super::super::BackgroundShellJobState;
use super::super::BackgroundShellJobStatus;
use super::super::BackgroundShellManager;
use super::super::BackgroundShellOutputLine;
use super::super::BackgroundShellReadyWaitOutcome;
use super::super::BackgroundShellServiceReadiness;
use super::super::DEFAULT_READY_WAIT_TIMEOUT_MS;
use super::super::MAX_STORED_LINES;
use super::super::READY_WAIT_POLL_INTERVAL_MS;
use super::super::apply_recipe_arguments_to_action;
use super::super::interaction_action_summary;
use super::super::invoke_http_recipe;
use super::super::invoke_redis_recipe;
use super::super::invoke_tcp_recipe;
use super::super::render_recipe_parameters;
use super::super::resolve_recipe_arguments;
use super::super::service_readiness_for_state;
use super::super::terminate_pid;

impl BackgroundShellManager {
    pub(crate) fn terminate_job(&self, job_id: &str) -> Result<(), String> {
        let job = self.lookup_job(job_id)?;
        let pid = {
            let state = job.lock().expect("background shell job lock");
            if !matches!(state.status, BackgroundShellJobStatus::Running) {
                return Ok(());
            }
            state.pid
        };
        terminate_pid(pid)?;
        let mut state = job.lock().expect("background shell job lock");
        state.status = BackgroundShellJobStatus::Terminated(None);
        state.stdin = None;
        Ok(())
    }

    pub(crate) fn send_input_to_job(
        &self,
        job_id: &str,
        text: &str,
        append_newline: bool,
    ) -> Result<usize, String> {
        let job = self.lookup_job(job_id)?;
        let mut state = job.lock().expect("background shell job lock");
        if !matches!(state.status, BackgroundShellJobStatus::Running) {
            return Err(format!("background shell job `{job_id}` is not running"));
        }
        let stdin = state
            .stdin
            .as_mut()
            .ok_or_else(|| format!("background shell job `{job_id}` is not accepting stdin"))?;
        let mut payload = text.as_bytes().to_vec();
        if append_newline {
            payload.push(b'\n');
        }
        stdin
            .write_all(&payload)
            .map_err(|err| format!("failed to write to background shell job `{job_id}`: {err}"))?;
        stdin.flush().map_err(|err| {
            format!("failed to flush background shell job `{job_id}` stdin: {err}")
        })?;
        Ok(payload.len())
    }

    pub(crate) fn service_attachment_summary(&self, job_id: &str) -> Result<String, String> {
        let job = self.lookup_job(job_id)?;
        let state = job.lock().expect("background shell job lock");
        if state.intent != BackgroundShellIntent::Service {
            return Err(format!(
                "background shell job `{job_id}` is not a service shell"
            ));
        }
        let mut lines = vec![
            format!("Service job: {}", state.id),
            format!(
                "State: {}",
                service_readiness_for_state(&state)
                    .expect("service readiness")
                    .as_str()
            ),
            format!("Command: {}", state.command),
        ];
        if let Some(label) = state.label.as_deref() {
            lines.push(format!("Label: {label}"));
        }
        if let Some(alias) = state.alias.as_deref() {
            lines.push(format!("Alias: {alias}"));
        }
        if !state.service_capabilities.is_empty() {
            lines.push(format!(
                "Capabilities: {}",
                state.service_capabilities.join(", ")
            ));
        }
        if let Some(protocol) = state.service_protocol.as_deref() {
            lines.push(format!("Protocol: {protocol}"));
        }
        if let Some(endpoint) = state.service_endpoint.as_deref() {
            lines.push(format!("Endpoint: {endpoint}"));
        }
        if let Some(attach_hint) = state.attach_hint.as_deref() {
            lines.push(format!("Attach hint: {attach_hint}"));
        }
        if !state.interaction_recipes.is_empty() {
            lines.push("Recipes:".to_string());
            for recipe in &state.interaction_recipes {
                let mut line = format!(
                    "- {} [{}]",
                    recipe.name,
                    interaction_action_summary(&recipe.action)
                );
                if let Some(description) = recipe.description.as_deref() {
                    line.push_str(&format!(": {description}"));
                }
                lines.push(line);
                if !recipe.parameters.is_empty() {
                    lines.push(format!(
                        "  params: {}",
                        render_recipe_parameters(&recipe.parameters)
                    ));
                }
                if let Some(example) = recipe.example.as_deref() {
                    lines.push(format!("  example: {example}"));
                }
            }
        }
        if let Some(ready_pattern) = state.ready_pattern.as_deref() {
            lines.push(format!("Ready pattern: {ready_pattern}"));
        }
        if state.service_capabilities.is_empty()
            && state.service_protocol.is_none()
            && state.service_endpoint.is_none()
            && state.attach_hint.is_none()
            && state.interaction_recipes.is_empty()
        {
            lines.push(
                "No explicit service attachment metadata has been declared for this job."
                    .to_string(),
            );
        }
        Ok(lines.join("\n"))
    }

    pub(crate) fn invoke_recipe(
        &self,
        job_id: &str,
        recipe_name: &str,
        args: &HashMap<String, String>,
        wait_for_ready_ms: Option<u64>,
    ) -> Result<String, String> {
        let (job_label, action, endpoint, parameters, has_ready_pattern) = {
            let job = self.lookup_job(job_id)?;
            let state = job.lock().expect("background shell job lock");
            if state.intent != BackgroundShellIntent::Service {
                return Err(format!(
                    "background shell job `{job_id}` is not a service shell"
                ));
            }
            let recipe = state
                .interaction_recipes
                .iter()
                .find(|recipe| recipe.name == recipe_name)
                .cloned()
                .ok_or_else(|| {
                    format!("background shell job `{job_id}` has no recipe named `{recipe_name}`")
                })?;
            (
                state.alias.clone().unwrap_or_else(|| state.id.clone()),
                recipe.action,
                state.service_endpoint.clone(),
                recipe.parameters,
                state.ready_pattern.is_some(),
            )
        };
        let resolved_args = resolve_recipe_arguments(&parameters, args)?;
        let action = apply_recipe_arguments_to_action(action, &resolved_args)?;
        let readiness_note = if has_ready_pattern
            && matches!(
                action,
                BackgroundShellInteractionAction::Http { .. }
                    | BackgroundShellInteractionAction::Tcp { .. }
                    | BackgroundShellInteractionAction::Redis { .. }
            ) {
            let wait_timeout_ms = wait_for_ready_ms.unwrap_or(DEFAULT_READY_WAIT_TIMEOUT_MS);
            if wait_timeout_ms == 0 {
                None
            } else {
                match self.wait_for_service_ready(job_id, wait_timeout_ms)? {
                    BackgroundShellReadyWaitOutcome::AlreadyReady => None,
                    BackgroundShellReadyWaitOutcome::BecameReady { waited_ms } => Some(format!(
                        "Readiness: waited {waited_ms}ms for service readiness."
                    )),
                }
            }
        } else {
            None
        };

        match action {
            BackgroundShellInteractionAction::Informational => Err(format!(
                "recipe `{recipe_name}` on background shell job `{job_id}` is descriptive only and does not declare an executable action"
            )),
            BackgroundShellInteractionAction::Stdin {
                text,
                append_newline,
            } => {
                let bytes_written = self.send_input_to_job(job_id, &text, append_newline)?;
                let mut lines = vec![
                    format!("Invoked recipe `{recipe_name}` on background shell job {job_label}."),
                    format!(
                        "Action: {}",
                        interaction_action_summary(&BackgroundShellInteractionAction::Stdin {
                            text,
                            append_newline,
                        })
                    ),
                ];
                if let Some(note) = readiness_note {
                    lines.push(note);
                }
                lines.push(format!(
                    "Sent {bytes_written} byte{} to stdin.",
                    if bytes_written == 1 { "" } else { "s" }
                ));
                Ok(lines.join("\n"))
            }
            BackgroundShellInteractionAction::Http {
                method,
                path,
                body,
                headers,
                expected_status,
            } => {
                let endpoint = endpoint.ok_or_else(|| {
                    format!(
                        "recipe `{recipe_name}` on background shell job `{job_id}` requires a service `endpoint`"
                    )
                })?;
                let response = invoke_http_recipe(
                    &endpoint,
                    &method,
                    &path,
                    body.as_deref(),
                    &headers,
                    expected_status,
                )?;
                let mut lines = vec![
                    format!("Invoked recipe `{recipe_name}` on background shell job {job_label}."),
                    format!(
                        "Action: {}",
                        interaction_action_summary(&BackgroundShellInteractionAction::Http {
                            method,
                            path,
                            body,
                            headers,
                            expected_status,
                        })
                    ),
                ];
                if let Some(note) = readiness_note {
                    lines.push(note);
                }
                lines.push("Response:".to_string());
                lines.push(response);
                Ok(lines.join("\n"))
            }
            BackgroundShellInteractionAction::Tcp {
                payload,
                append_newline,
                expect_substring,
                read_timeout_ms,
            } => {
                let endpoint = endpoint.ok_or_else(|| {
                    format!(
                        "recipe `{recipe_name}` on background shell job `{job_id}` requires a service `endpoint`"
                    )
                })?;
                let response = invoke_tcp_recipe(
                    &endpoint,
                    payload.as_deref(),
                    append_newline,
                    expect_substring.as_deref(),
                    read_timeout_ms,
                )?;
                let mut lines = vec![
                    format!("Invoked recipe `{recipe_name}` on background shell job {job_label}."),
                    format!(
                        "Action: {}",
                        interaction_action_summary(&BackgroundShellInteractionAction::Tcp {
                            payload,
                            append_newline,
                            expect_substring,
                            read_timeout_ms,
                        })
                    ),
                ];
                if let Some(note) = readiness_note {
                    lines.push(note);
                }
                lines.push("Response:".to_string());
                lines.push(response);
                Ok(lines.join("\n"))
            }
            BackgroundShellInteractionAction::Redis {
                command,
                expect_substring,
                read_timeout_ms,
            } => {
                let endpoint = endpoint.ok_or_else(|| {
                    format!(
                        "recipe `{recipe_name}` on background shell job `{job_id}` requires a service `endpoint`"
                    )
                })?;
                let response = invoke_redis_recipe(
                    &endpoint,
                    &command,
                    expect_substring.as_deref(),
                    read_timeout_ms,
                )?;
                let mut lines = vec![
                    format!("Invoked recipe `{recipe_name}` on background shell job {job_label}."),
                    format!(
                        "Action: {}",
                        interaction_action_summary(&BackgroundShellInteractionAction::Redis {
                            command,
                            expect_substring,
                            read_timeout_ms,
                        })
                    ),
                ];
                if let Some(note) = readiness_note {
                    lines.push(note);
                }
                lines.push("Response:".to_string());
                lines.push(response);
                Ok(lines.join("\n"))
            }
        }
    }

    pub(crate) fn wait_for_service_ready(
        &self,
        job_id: &str,
        timeout_ms: u64,
    ) -> Result<BackgroundShellReadyWaitOutcome, String> {
        let start = Instant::now();
        loop {
            let job = self.lookup_job(job_id)?;
            let state = job.lock().expect("background shell job lock");
            if state.intent != BackgroundShellIntent::Service {
                return Err(format!(
                    "background shell job `{job_id}` is not a service shell"
                ));
            }
            let readiness = service_readiness_for_state(&state).expect("service readiness");
            match readiness {
                BackgroundShellServiceReadiness::Ready => {
                    let waited_ms = start.elapsed().as_millis() as u64;
                    return Ok(if waited_ms == 0 {
                        BackgroundShellReadyWaitOutcome::AlreadyReady
                    } else {
                        BackgroundShellReadyWaitOutcome::BecameReady { waited_ms }
                    });
                }
                BackgroundShellServiceReadiness::Untracked => {
                    return Err(format!(
                        "background shell job `{job_id}` does not declare a `readyPattern`; readiness is untracked"
                    ));
                }
                BackgroundShellServiceReadiness::Booting => {
                    if !matches!(state.status, BackgroundShellJobStatus::Running) {
                        return Err(format!(
                            "background shell job `{job_id}` stopped before reaching its ready pattern"
                        ));
                    }
                }
            }
            drop(state);
            let waited_ms = start.elapsed().as_millis() as u64;
            if waited_ms >= timeout_ms {
                return Err(format!(
                    "background shell job `{job_id}` did not become ready within {timeout_ms}ms"
                ));
            }
            let remaining_ms = timeout_ms.saturating_sub(waited_ms);
            thread::sleep(Duration::from_millis(
                READY_WAIT_POLL_INTERVAL_MS.min(remaining_ms.max(1)),
            ));
        }
    }

    pub(crate) fn resolve_service_capability_reference(
        &self,
        capability: &str,
    ) -> Result<String, String> {
        let capability = validate_service_capability(capability)?;
        let matches = self
            .running_service_snapshots()
            .into_iter()
            .filter(|job| {
                job.service_capabilities
                    .iter()
                    .any(|entry| entry == &capability)
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(format!(
                "unknown running background shell capability `@{capability}`; use :ps capabilities to inspect reusable service roles"
            )),
            [job] => Ok(job.id.clone()),
            jobs => {
                let refs = jobs
                    .iter()
                    .map(|job| match job.alias.as_deref() {
                        Some(alias) => format!("{} ({alias})", job.id),
                        None => job.id.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(format!(
                    "background shell capability `@{capability}` is ambiguous across multiple running service jobs: {refs}; use :ps capabilities to inspect reusable service roles"
                ))
            }
        }
    }
}

pub(super) fn resolve_background_cwd(
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

pub(super) fn parse_background_shell_intent(
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

pub(super) fn parse_background_shell_label(value: Option<&serde_json::Value>) -> Option<String> {
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

pub(super) fn parse_background_shell_ready_pattern(
    value: Option<&serde_json::Value>,
) -> Result<Option<String>, String> {
    parse_background_shell_optional_string(value, "readyPattern")
}

pub(super) fn parse_background_shell_capabilities(
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

pub(super) fn parse_background_shell_timeout_ms(
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

pub(super) fn validate_alias(alias: &str) -> Result<String, String> {
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

pub(super) fn spawn_shell_process(
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

pub(super) fn spawn_output_reader<R>(
    reader: R,
    job: Arc<Mutex<BackgroundShellJobState>>,
    stream_name: Option<&'static str>,
) where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            match line {
                Ok(line) => append_output_line(&job, stream_name, &line),
                Err(err) => {
                    append_output_line(
                        &job,
                        Some("stderr"),
                        &format!("background shell reader error: {err}"),
                    );
                    break;
                }
            }
        }
    });
}

fn append_output_line(
    job: &Arc<Mutex<BackgroundShellJobState>>,
    stream_name: Option<&'static str>,
    line: &str,
) {
    let text = if let Some(stream_name) = stream_name {
        format!("[{stream_name}] {line}")
    } else {
        line.to_string()
    };
    let mut state = job.lock().expect("background shell job lock");
    state.total_lines += 1;
    let cursor = state.total_lines;
    if !state.service_ready
        && let Some(pattern) = state.ready_pattern.as_deref()
        && (line.contains(pattern) || text.contains(pattern))
    {
        state.service_ready = true;
    }
    state
        .lines
        .push_back(BackgroundShellOutputLine { cursor, text });
    if state.lines.len() > MAX_STORED_LINES {
        state.lines.pop_front();
    }
}

pub(crate) fn terminate_jobs(manager: &BackgroundShellManager, job_ids: Vec<String>) -> usize {
    let mut terminated = 0;
    for job_id in job_ids {
        if manager.terminate_job(&job_id).is_ok() {
            terminated += 1;
        }
    }
    terminated
}
