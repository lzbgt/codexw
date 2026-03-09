use std::collections::HashMap;

use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellInteractionAction;
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellReadyWaitOutcome;
use crate::background_shells::DEFAULT_READY_WAIT_TIMEOUT_MS;
use crate::background_shells::recipes::apply_recipe_arguments_to_action;
use crate::background_shells::recipes::interaction_action_summary;
use crate::background_shells::recipes::invoke_http_recipe;
use crate::background_shells::recipes::invoke_redis_recipe;
use crate::background_shells::recipes::invoke_tcp_recipe;
use crate::background_shells::recipes::resolve_recipe_arguments;

impl BackgroundShellManager {
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
}
