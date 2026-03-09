use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::thread;

use super::runtime::parse_background_shell_capabilities;
use super::runtime::parse_background_shell_intent;
use super::runtime::parse_background_shell_label;
use super::runtime::parse_background_shell_optional_string;
use super::runtime::parse_background_shell_ready_pattern;
use super::runtime::resolve_background_cwd;
use super::runtime::spawn_output_reader;
use super::runtime::spawn_shell_process;
use super::runtime::validate_alias;
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
                        BackgroundShellJobStatus::Terminated(exit_code)
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

    pub(crate) fn list_from_tool(&self) -> String {
        let snapshots = self.snapshots();
        if snapshots.is_empty() {
            return "No background shell jobs.".to_string();
        }
        let mut lines = vec!["Background shell jobs:".to_string()];
        for snapshot in snapshots {
            let mut line = format!(
                "{}  {}  intent={}  pid={}  {}",
                snapshot.id,
                snapshot.status,
                snapshot.intent.as_str(),
                snapshot.pid,
                snapshot.command
            );
            if let Some(exit_code) = snapshot.exit_code {
                line.push_str(&format!("  exit={exit_code}"));
            }
            if let Some(label) = snapshot.label.as_deref() {
                line.push_str(&format!("  label={label}"));
            }
            if let Some(alias) = snapshot.alias.as_deref() {
                line.push_str(&format!("  alias={alias}"));
            }
            if !snapshot.service_capabilities.is_empty() {
                line.push_str(&format!(
                    "  caps={}",
                    snapshot.service_capabilities.join(",")
                ));
            }
            if !snapshot.dependency_capabilities.is_empty() {
                line.push_str(&format!(
                    "  depends={}",
                    snapshot
                        .dependency_capabilities
                        .iter()
                        .map(|capability| format!("@{capability}"))
                        .collect::<Vec<_>>()
                        .join(",")
                ));
            }
            if let Some(protocol) = snapshot.service_protocol.as_deref() {
                line.push_str(&format!("  protocol={protocol}"));
            }
            if let Some(endpoint) = snapshot.service_endpoint.as_deref() {
                line.push_str(&format!("  endpoint={endpoint}"));
            }
            if !snapshot.interaction_recipes.is_empty() {
                line.push_str(&format!("  recipes={}", snapshot.interaction_recipes.len()));
            }
            if let Some(service_readiness) = snapshot.service_readiness {
                line.push_str(&format!("  service={}", service_readiness.as_str()));
            }
            if let Some(source_thread_id) = snapshot.origin.source_thread_id.as_deref() {
                line.push_str(&format!("  source={source_thread_id}"));
            }
            if snapshot.status == "failed" && !snapshot.recent_lines.is_empty() {
                line.push_str(&format!("  {}", snapshot.recent_lines.join(" | ")));
            }
            lines.push(line);
        }
        lines.push(
            "Use background_shell_poll with a jobId to inspect logs while continuing other work."
                .to_string(),
        );
        if let Some(capability_lines) = self.render_service_capability_index_lines() {
            lines.extend(capability_lines);
        }
        let conflicts = self.service_capability_conflicts();
        if !conflicts.is_empty() {
            lines.push("Capability conflicts:".to_string());
            for (capability, jobs) in conflicts {
                lines.push(format!("@{capability} -> {}", jobs.join(", ")));
            }
        }
        lines.join("\n")
    }

    pub(crate) fn terminate_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_terminate expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_terminate requires `jobId`".to_string())?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        self.terminate_job(&resolved_job_id)?;
        Ok(format!(
            "Termination requested for background shell job {resolved_job_id}."
        ))
    }

    pub(crate) fn clean_from_tool(&self, arguments: &serde_json::Value) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_clean expects an object argument".to_string())?;
        let scope = object
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("all");
        let capability = object.get("capability").and_then(serde_json::Value::as_str);
        let terminated = match scope {
            "all" => {
                if capability.is_some() {
                    return Err(
                        "background_shell_clean `capability` is only valid with `scope=blockers` or `scope=services`"
                            .to_string(),
                    );
                }
                self.terminate_all_running()
            }
            "blockers" => match capability {
                Some(capability) => self.terminate_running_blockers_by_capability(capability)?,
                None => self.terminate_running_by_intent(BackgroundShellIntent::Prerequisite),
            },
            "shells" => {
                if capability.is_some() {
                    return Err(
                        "background_shell_clean `capability` is only valid with `scope=blockers` or `scope=services`"
                            .to_string(),
                    );
                }
                self.terminate_all_running()
            }
            "services" => match capability {
                Some(capability) => self.terminate_running_services_by_capability(capability)?,
                None => self.terminate_running_by_intent(BackgroundShellIntent::Service),
            },
            other => {
                return Err(format!(
                    "background_shell_clean `scope` must be one of `all`, `blockers`, `shells`, or `services`, got `{other}`"
                ));
            }
        };
        let summary = match (scope, capability) {
            ("blockers", Some(capability)) => format!(
                "Terminated {terminated} blocking prerequisite background shell job{} for reusable capability dependency @{}.",
                if terminated == 1 { "" } else { "s" },
                capability.trim_start_matches('@')
            ),
            ("services", Some(capability)) => format!(
                "Terminated {terminated} background shell job{} for reusable service capability @{}.",
                if terminated == 1 { "" } else { "s" },
                capability.trim_start_matches('@')
            ),
            ("services", None) => format!(
                "Terminated {terminated} service background shell job{}.",
                if terminated == 1 { "" } else { "s" }
            ),
            ("blockers", None) => format!(
                "Terminated {terminated} blocking prerequisite background shell job{}.",
                if terminated == 1 { "" } else { "s" }
            ),
            ("all" | "shells", None) => format!(
                "Terminated {terminated} local background shell job{}.",
                if terminated == 1 { "" } else { "s" }
            ),
            _ => unreachable!(),
        };
        Ok(summary)
    }

    pub(crate) fn resolve_job_reference(&self, reference: &str) -> Result<String, String> {
        let reference = reference.trim();
        if reference.is_empty() {
            return Err("background shell job reference cannot be empty".to_string());
        }
        if reference.starts_with("bg-") {
            self.lookup_job(reference)?;
            return Ok(reference.to_string());
        }
        if let Some(capability) = reference.strip_prefix('@') {
            return self.resolve_service_capability_reference(capability);
        }
        if let Some(job_id) = self
            .snapshots()
            .into_iter()
            .find(|job| job.alias.as_deref() == Some(reference))
            .map(|job| job.id)
        {
            return Ok(job_id);
        }
        let index = reference
            .parse::<usize>()
            .map_err(|_| format!("unknown background shell job `{reference}`"))?;
        if index == 0 {
            return Err("background shell job index must be 1-based".to_string());
        }
        let snapshots = self.snapshots();
        snapshots
            .get(index - 1)
            .map(|job| job.id.clone())
            .ok_or_else(|| format!("unknown background shell job `{reference}`"))
    }

    pub(crate) fn set_job_alias(&self, job_id: &str, alias: &str) -> Result<(), String> {
        let alias = validate_alias(alias)?;
        let jobs = self.inner.jobs.lock().expect("background shell jobs lock");
        for job in jobs.values() {
            let state = job.lock().expect("background shell job lock");
            if state.id != job_id && state.alias.as_deref() == Some(alias.as_str()) {
                return Err(format!(
                    "background shell alias `{alias}` is already in use"
                ));
            }
        }
        let job = jobs
            .get(job_id)
            .cloned()
            .ok_or_else(|| format!("unknown background shell job `{job_id}`"))?;
        drop(jobs);
        let mut state = job.lock().expect("background shell job lock");
        state.alias = Some(alias);
        Ok(())
    }

    pub(crate) fn clear_job_alias(&self, alias: &str) -> Result<String, String> {
        let alias = validate_alias(alias)?;
        let jobs = self.inner.jobs.lock().expect("background shell jobs lock");
        let job = jobs
            .values()
            .find_map(|job| {
                let state = job.lock().expect("background shell job lock");
                (state.alias.as_deref() == Some(alias.as_str())).then_some(job.clone())
            })
            .ok_or_else(|| format!("unknown background shell alias `{alias}`"))?;
        drop(jobs);
        let mut state = job.lock().expect("background shell job lock");
        let job_id = state.id.clone();
        state.alias = None;
        Ok(job_id)
    }

    pub(crate) fn clear_job_alias_for_job(&self, job_id: &str) -> Result<(), String> {
        let job = self.lookup_job(job_id)?;
        let mut state = job.lock().expect("background shell job lock");
        state.alias = None;
        Ok(())
    }

    pub(crate) fn update_alias_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_set_alias expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_set_alias requires `jobId`".to_string())?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        match object.get("alias") {
            Some(serde_json::Value::Null) => {
                self.clear_job_alias_for_job(&resolved_job_id)?;
                Ok(format!(
                    "Cleared alias for background shell job {resolved_job_id}."
                ))
            }
            Some(serde_json::Value::String(alias)) => {
                self.set_job_alias(&resolved_job_id, alias)?;
                Ok(format!(
                    "Aliased background shell job {resolved_job_id} as {}.",
                    validate_alias(alias)?
                ))
            }
            Some(_) => {
                Err("background_shell_set_alias `alias` must be a string or null".to_string())
            }
            None => Err("background_shell_set_alias requires `alias`".to_string()),
        }
    }

    pub(crate) fn terminate_job_for_operator(&self, job_id: &str) -> Result<(), String> {
        self.terminate_job(job_id)
    }
}
