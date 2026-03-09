use crate::background_shells::BackgroundShellJobStatus;
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::DEFAULT_POLL_LIMIT;
use crate::background_shells::MAX_POLL_LIMIT;
use crate::background_shells::exit_code;
use crate::background_shells::recipes::interaction_action_summary;
use crate::background_shells::recipes::parse_recipe_arguments_map;

use super::super::super::service_readiness_for_state;
use super::super::super::status_label;
use super::super::runtime::parse_background_shell_timeout_ms;

impl BackgroundShellManager {
    pub(crate) fn poll_from_tool(&self, arguments: &serde_json::Value) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_poll expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_poll requires `jobId`".to_string())?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        let after_line = object
            .get("afterLine")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let limit = object
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .map(|value| value.clamp(1, MAX_POLL_LIMIT))
            .unwrap_or(DEFAULT_POLL_LIMIT);
        let job = self.lookup_job(&resolved_job_id)?;
        let state = job.lock().expect("background shell job lock");
        let matching = state
            .lines
            .iter()
            .filter(|line| line.cursor > after_line)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();

        let mut lines = vec![
            format!("Job: {}", state.id),
            format!("Status: {}", status_label(&state.status)),
            format!("PID: {}", state.pid),
            format!("CWD: {}", state.cwd),
            format!("Intent: {}", state.intent.as_str()),
            format!("Command: {}", state.command),
            format!("Next afterLine: {}", state.total_lines),
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
        if !state.dependency_capabilities.is_empty() {
            lines.push(format!(
                "Depends on capabilities: {}",
                state
                    .dependency_capabilities
                    .iter()
                    .map(|capability| format!("@{capability}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(service_protocol) = state.service_protocol.as_deref() {
            lines.push(format!("Protocol: {service_protocol}"));
        }
        if let Some(service_endpoint) = state.service_endpoint.as_deref() {
            lines.push(format!("Endpoint: {service_endpoint}"));
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
                if let Some(example) = recipe.example.as_deref() {
                    lines.push(format!("  example: {example}"));
                }
            }
        }
        if let Some(ready_pattern) = state.ready_pattern.as_deref() {
            lines.push(format!("Ready pattern: {ready_pattern}"));
        }
        if let Some(service_readiness) = service_readiness_for_state(&state) {
            lines.push(format!("Service state: {}", service_readiness.as_str()));
        }
        if let Some(source_thread_id) = state.origin.source_thread_id.as_deref() {
            lines.push(format!("Source thread: {source_thread_id}"));
        }
        if let Some(source_call_id) = state.origin.source_call_id.as_deref() {
            lines.push(format!("Source call: {source_call_id}"));
        }
        if let Some(source_tool) = state.origin.source_tool.as_deref() {
            lines.push(format!("Source tool: {source_tool}"));
        }
        if let Some(exit_code) = exit_code(&state.status) {
            lines.push(format!("Exit code: {exit_code}"));
        }
        if let BackgroundShellJobStatus::Failed(message) = &state.status {
            lines.push(format!("Error: {message}"));
        }
        if matching.is_empty() {
            lines.push("Output: (no new lines)".to_string());
        } else {
            lines.push("Output:".to_string());
            for line in matching {
                lines.push(format!("{:>4} | {}", line.cursor, line.text));
            }
        }
        Ok(lines.join("\n"))
    }

    pub(crate) fn send_input_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_send expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_send requires `jobId`".to_string())?;
        let text = object
            .get("text")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_send requires `text`".to_string())?;
        let append_newline = object
            .get("appendNewline")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        let bytes_written = self.send_input_to_job(&resolved_job_id, text, append_newline)?;
        Ok(format!(
            "Sent {bytes_written} byte{} to background shell job {resolved_job_id}.",
            if bytes_written == 1 { "" } else { "s" }
        ))
    }

    pub(crate) fn attach_from_tool(&self, arguments: &serde_json::Value) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_attach expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_attach requires `jobId`".to_string())?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        self.service_attachment_summary(&resolved_job_id)
    }

    pub(crate) fn inspect_capability_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object().ok_or_else(|| {
            "background_shell_inspect_capability expects an object argument".to_string()
        })?;
        let capability = object
            .get("capability")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                "background_shell_inspect_capability requires `capability`".to_string()
            })?;
        Ok(self
            .render_single_service_capability_for_ps(capability)?
            .join("\n"))
    }

    pub(crate) fn list_capabilities_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object();
        let issue_filter = super::super::super::services::parse_capability_issue_filter(
            object
                .and_then(|object| object.get("status"))
                .and_then(serde_json::Value::as_str),
            "background_shell_list_capabilities",
        )?;
        let rendered = self
            .render_service_capabilities_for_ps_filtered(issue_filter)
            .ok_or_else(|| "No reusable service capabilities tracked right now.".to_string())?;
        Ok(rendered.join("\n"))
    }

    pub(crate) fn wait_ready_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_wait_ready expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_wait_ready requires `jobId`".to_string())?;
        let timeout_ms = parse_background_shell_timeout_ms(
            object.get("timeoutMs"),
            "background_shell_wait_ready",
        )?
        .unwrap_or(crate::background_shells::DEFAULT_READY_WAIT_TIMEOUT_MS);
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        self.wait_ready_for_operator(&resolved_job_id, timeout_ms)
    }

    pub(crate) fn invoke_recipe_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments.as_object().ok_or_else(|| {
            "background_shell_invoke_recipe expects an object argument".to_string()
        })?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_invoke_recipe requires `jobId`".to_string())?;
        let recipe_name = object
            .get("recipe")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "background_shell_invoke_recipe requires `recipe`".to_string())?;
        let wait_for_ready_ms = parse_background_shell_timeout_ms(
            object.get("waitForReadyMs"),
            "background_shell_invoke_recipe",
        )?;
        let args =
            parse_recipe_arguments_map(object.get("args"), "background_shell_invoke_recipe")?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        self.invoke_recipe(&resolved_job_id, recipe_name, &args, wait_for_ready_ms)
    }
}
