use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

use super::super::super::BackgroundShellIntent;
use super::super::super::BackgroundShellInteractionAction;
use super::super::super::BackgroundShellJobStatus;
use super::super::super::BackgroundShellManager;
use super::super::super::BackgroundShellReadyWaitOutcome;
use super::super::super::BackgroundShellServiceReadiness;
use super::super::super::DEFAULT_READY_WAIT_TIMEOUT_MS;
use super::super::super::READY_WAIT_POLL_INTERVAL_MS;
use super::super::super::apply_recipe_arguments_to_action;
use super::super::super::interaction_action_summary;
use super::super::super::invoke_http_recipe;
use super::super::super::invoke_redis_recipe;
use super::super::super::invoke_tcp_recipe;
use super::super::super::render_recipe_parameters;
use super::super::super::resolve_recipe_arguments;
use super::super::super::service_readiness_for_state;
use super::process::validate_service_capability;

impl BackgroundShellManager {
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
            std::thread::sleep(Duration::from_millis(
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
