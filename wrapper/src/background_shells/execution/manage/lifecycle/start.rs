use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::thread;

use super::super::super::runtime::parse_background_shell_capabilities;
use super::super::super::runtime::parse_background_shell_intent;
use super::super::super::runtime::parse_background_shell_label;
use super::super::super::runtime::parse_background_shell_optional_string;
use super::super::super::runtime::parse_background_shell_ready_pattern;
use super::super::super::runtime::resolve_background_cwd;
use super::super::super::runtime::spawn_output_reader;
use super::super::super::runtime::spawn_shell_process;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellJobState;
use crate::background_shells::BackgroundShellJobStatus;
use crate::background_shells::BackgroundShellManager;
use crate::background_shells::BackgroundShellOrigin;
use crate::background_shells::parse_background_shell_interaction_recipes;

impl BackgroundShellManager {
    #[cfg(test)]
    pub(crate) fn start_from_tool(
        &self,
        arguments: &serde_json::Value,
        resolved_cwd: &str,
    ) -> Result<String, String> {
        self.start_from_tool_with_context(arguments, resolved_cwd, BackgroundShellOrigin::default())
    }

    pub(crate) fn start_from_tool_with_context(
        &self,
        arguments: &serde_json::Value,
        resolved_cwd: &str,
        origin: BackgroundShellOrigin,
    ) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_start expects an object argument".to_string())?;
        let command = object
            .get("command")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "background_shell_start requires a non-empty `command`".to_string())?;
        let cwd = resolve_background_cwd(
            object.get("cwd").and_then(serde_json::Value::as_str),
            resolved_cwd,
        )?;
        let intent = parse_background_shell_intent(object.get("intent"))?;
        let label = parse_background_shell_label(object.get("label"));
        let service_capabilities =
            parse_background_shell_capabilities(object.get("capabilities"), "capabilities")?;
        let dependency_capabilities = parse_background_shell_capabilities(
            object.get("dependsOnCapabilities"),
            "dependsOnCapabilities",
        )?;
        let service_protocol =
            parse_background_shell_optional_string(object.get("protocol"), "protocol")?;
        let service_endpoint =
            parse_background_shell_optional_string(object.get("endpoint"), "endpoint")?;
        let attach_hint =
            parse_background_shell_optional_string(object.get("attachHint"), "attachHint")?;
        let interaction_recipes =
            parse_background_shell_interaction_recipes(object.get("recipes"))?;
        let ready_pattern = parse_background_shell_ready_pattern(object.get("readyPattern"))?;
        let has_service_contract = ready_pattern.is_some()
            || !service_capabilities.is_empty()
            || service_protocol.is_some()
            || service_endpoint.is_some()
            || attach_hint.is_some()
            || !interaction_recipes.is_empty();
        if has_service_contract && intent != BackgroundShellIntent::Service {
            return Err(
                "background_shell_start service contract fields (`readyPattern`, `capabilities`, `protocol`, `endpoint`, `attachHint`, `recipes`) are only supported when `intent=service`".to_string(),
            );
        }
        let job_id = format!(
            "bg-{}",
            self.inner.next_job_id.fetch_add(1, Ordering::Relaxed) + 1
        );
        let mut process = spawn_shell_process(command, &cwd)?;
        let pid = process.id();
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| "background shell stdout pipe was not available".to_string())?;
        let stderr = process
            .stderr
            .take()
            .ok_or_else(|| "background shell stderr pipe was not available".to_string())?;
        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| "background shell stdin pipe was not available".to_string())?;
        let job = Arc::new(Mutex::new(BackgroundShellJobState {
            id: job_id.clone(),
            pid,
            command: command.to_string(),
            cwd: cwd.display().to_string(),
            intent,
            label: label.clone(),
            alias: None,
            service_capabilities: service_capabilities.clone(),
            dependency_capabilities: dependency_capabilities.clone(),
            service_protocol: service_protocol.clone(),
            service_endpoint: service_endpoint.clone(),
            attach_hint: attach_hint.clone(),
            interaction_recipes: interaction_recipes.clone(),
            ready_pattern: ready_pattern.clone(),
            service_ready: false,
            origin: origin.clone(),
            stdin: Some(stdin),
            status: BackgroundShellJobStatus::Running,
            total_lines: 0,
            last_output_at: None,
            lines: Default::default(),
        }));

        self.inner
            .jobs
            .lock()
            .expect("background shell jobs lock")
            .insert(job_id.clone(), Arc::clone(&job));

        spawn_output_reader(stdout, Arc::clone(&job), None);
        spawn_output_reader(stderr, Arc::clone(&job), Some("stderr"));
        thread::spawn(move || {
            let status = match process.wait() {
                Ok(status) => {
                    let exit_code = status.code();
                    if status.success() {
                        BackgroundShellJobStatus::Completed(exit_code.unwrap_or(0))
                    } else {
                        BackgroundShellJobStatus::Completed(exit_code.unwrap_or(1))
                    }
                }
                Err(err) => BackgroundShellJobStatus::Failed(err.to_string()),
            };
            let mut state = job.lock().expect("background shell job lock");
            state.status = status;
            state.stdin = None;
        });

        let mut lines = vec![
            format!("Started background shell job {job_id}"),
            format!("PID: {pid}"),
            format!("CWD: {}", cwd.display()),
            format!("Intent: {}", intent.as_str()),
            format!("Command: {command}"),
        ];
        if let Some(label) = label {
            lines.insert(4, format!("Label: {label}"));
        }
        if !service_capabilities.is_empty() {
            lines.push(format!("Capabilities: {}", service_capabilities.join(", ")));
        }
        if !dependency_capabilities.is_empty() {
            lines.push(format!(
                "Depends on capabilities: {}",
                dependency_capabilities
                    .iter()
                    .map(|capability| format!("@{capability}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(service_protocol) = service_protocol.as_deref() {
            lines.push(format!("Protocol: {service_protocol}"));
        }
        if let Some(service_endpoint) = service_endpoint.as_deref() {
            lines.push(format!("Endpoint: {service_endpoint}"));
        }
        if let Some(attach_hint) = attach_hint.as_deref() {
            lines.push(format!("Attach hint: {attach_hint}"));
        }
        if !interaction_recipes.is_empty() {
            lines.push(format!("Recipes: {}", interaction_recipes.len()));
        }
        if let Some(ready_pattern) = ready_pattern.as_deref() {
            lines.push(format!("Ready pattern: {ready_pattern}"));
            lines.push("Service state: booting".to_string());
        } else if intent == BackgroundShellIntent::Service {
            lines.push("Service state: untracked".to_string());
        }
        lines.push(format!(
            "Use background_shell_poll with {{\"jobId\":\"{job_id}\"}} to inspect output while continuing other work."
        ));
        Ok(lines.join("\n"))
    }
}
