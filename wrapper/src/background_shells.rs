use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::ChildStdin;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[path = "background_shells/recipes.rs"]
mod recipes;
#[cfg(test)]
#[path = "background_shells/tests.rs"]
mod tests;

use self::recipes::apply_recipe_arguments_to_action;
use self::recipes::interaction_action_summary;
use self::recipes::invoke_http_recipe;
use self::recipes::invoke_redis_recipe;
use self::recipes::invoke_tcp_recipe;
use self::recipes::parse_background_shell_interaction_recipes;
use self::recipes::parse_recipe_arguments_map;
use self::recipes::render_recipe_parameters;
use self::recipes::resolve_recipe_arguments;

const DEFAULT_POLL_LIMIT: usize = 40;
const MAX_POLL_LIMIT: usize = 200;
const MAX_STORED_LINES: usize = 2_000;
const MAX_RENDERED_RECENT_LINES: usize = 3;
const DEFAULT_READY_WAIT_TIMEOUT_MS: u64 = 5_000;
const READY_WAIT_POLL_INTERVAL_MS: u64 = 25;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum BackgroundShellIntent {
    Prerequisite,
    #[default]
    Observation,
    Service,
}

impl BackgroundShellIntent {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Prerequisite => "prerequisite",
            Self::Observation => "observation",
            Self::Service => "service",
        }
    }

    pub(crate) fn is_blocking(self) -> bool {
        matches!(self, Self::Prerequisite)
    }

    fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "prerequisite" => Some(Self::Prerequisite),
            "observation" => Some(Self::Observation),
            "service" => Some(Self::Service),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundShellServiceReadiness {
    Booting,
    Ready,
    Untracked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackgroundShellReadyWaitOutcome {
    AlreadyReady,
    BecameReady { waited_ms: u64 },
}

impl BackgroundShellServiceReadiness {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Booting => "booting",
            Self::Ready => "ready",
            Self::Untracked => "untracked",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BackgroundShellOrigin {
    pub(crate) source_thread_id: Option<String>,
    pub(crate) source_call_id: Option<String>,
    pub(crate) source_tool: Option<String>,
}

#[derive(Clone, Default)]
pub(crate) struct BackgroundShellManager {
    inner: Arc<BackgroundShellManagerInner>,
}

#[derive(Default)]
struct BackgroundShellManagerInner {
    next_job_id: AtomicU64,
    jobs: Mutex<HashMap<String, Arc<Mutex<BackgroundShellJobState>>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackgroundShellJobSnapshot {
    pub(crate) id: String,
    pub(crate) pid: u32,
    pub(crate) command: String,
    pub(crate) cwd: String,
    pub(crate) intent: BackgroundShellIntent,
    pub(crate) label: Option<String>,
    pub(crate) alias: Option<String>,
    pub(crate) service_capabilities: Vec<String>,
    pub(crate) dependency_capabilities: Vec<String>,
    pub(crate) service_protocol: Option<String>,
    pub(crate) service_endpoint: Option<String>,
    pub(crate) attach_hint: Option<String>,
    pub(crate) interaction_recipes: Vec<BackgroundShellInteractionRecipe>,
    pub(crate) ready_pattern: Option<String>,
    pub(crate) service_readiness: Option<BackgroundShellServiceReadiness>,
    pub(crate) origin: BackgroundShellOrigin,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) total_lines: u64,
    pub(crate) recent_lines: Vec<String>,
}

#[derive(Debug)]
struct BackgroundShellJobState {
    id: String,
    pid: u32,
    command: String,
    cwd: String,
    intent: BackgroundShellIntent,
    label: Option<String>,
    alias: Option<String>,
    service_capabilities: Vec<String>,
    dependency_capabilities: Vec<String>,
    service_protocol: Option<String>,
    service_endpoint: Option<String>,
    attach_hint: Option<String>,
    interaction_recipes: Vec<BackgroundShellInteractionRecipe>,
    ready_pattern: Option<String>,
    service_ready: bool,
    origin: BackgroundShellOrigin,
    stdin: Option<ChildStdin>,
    status: BackgroundShellJobStatus,
    total_lines: u64,
    lines: VecDeque<BackgroundShellOutputLine>,
}

#[derive(Debug, Clone)]
struct BackgroundShellOutputLine {
    cursor: u64,
    text: String,
}

#[derive(Debug, Clone)]
enum BackgroundShellJobStatus {
    Running,
    Completed(i32),
    Failed(String),
    Terminated(Option<i32>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundShellInteractionRecipe {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) example: Option<String>,
    pub(crate) parameters: Vec<BackgroundShellInteractionParameter>,
    pub(crate) action: BackgroundShellInteractionAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundShellInteractionParameter {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) default: Option<String>,
    pub(crate) required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BackgroundShellInteractionAction {
    Informational,
    Stdin {
        text: String,
        append_newline: bool,
    },
    Http {
        method: String,
        path: String,
        body: Option<String>,
        headers: Vec<(String, String)>,
        expected_status: Option<u16>,
    },
    Tcp {
        payload: Option<String>,
        append_newline: bool,
        expect_substring: Option<String>,
        read_timeout_ms: Option<u64>,
    },
    Redis {
        command: Vec<String>,
        expect_substring: Option<String>,
        read_timeout_ms: Option<u64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundShellCapabilityDependencyState {
    Satisfied,
    Booting,
    Missing,
    Ambiguous,
}

impl BackgroundShellCapabilityDependencyState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Booting => "booting",
            Self::Missing => "missing",
            Self::Ambiguous => "ambiguous",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundShellCapabilityDependencySummary {
    pub(crate) job_id: String,
    pub(crate) job_alias: Option<String>,
    pub(crate) job_label: Option<String>,
    pub(crate) capability: String,
    pub(crate) blocking: bool,
    pub(crate) status: BackgroundShellCapabilityDependencyState,
    pub(crate) providers: Vec<String>,
}

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
        let service_capabilities = parse_background_shell_capabilities(object.get("capabilities"))?;
        let dependency_capabilities =
            parse_background_shell_capabilities(object.get("dependsOnCapabilities"))?;
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
            lines: VecDeque::new(),
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
        .unwrap_or(DEFAULT_READY_WAIT_TIMEOUT_MS);
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

    pub(crate) fn poll_job(
        &self,
        job_id: &str,
        after_line: u64,
        limit: usize,
    ) -> Result<String, String> {
        self.poll_from_tool(&serde_json::json!({
            "jobId": job_id,
            "afterLine": after_line,
            "limit": limit,
        }))
    }

    pub(crate) fn terminate_job_for_operator(&self, job_id: &str) -> Result<(), String> {
        self.terminate_job(job_id)
    }

    pub(crate) fn send_input_for_operator(
        &self,
        job_id: &str,
        text: &str,
        append_newline: bool,
    ) -> Result<usize, String> {
        self.send_input_to_job(job_id, text, append_newline)
    }

    pub(crate) fn attach_for_operator(&self, job_id: &str) -> Result<String, String> {
        self.service_attachment_summary(job_id)
    }

    pub(crate) fn wait_ready_for_operator(
        &self,
        job_id: &str,
        timeout_ms: u64,
    ) -> Result<String, String> {
        let outcome = self.wait_for_service_ready(job_id, timeout_ms)?;
        let job = self.lookup_job(job_id)?;
        let state = job.lock().expect("background shell job lock");
        let job_label = state.alias.clone().unwrap_or_else(|| state.id.clone());
        let ready_pattern = state.ready_pattern.clone().unwrap_or_default();
        let message = match outcome {
            BackgroundShellReadyWaitOutcome::AlreadyReady => {
                format!("Service background shell job {job_label} is already ready.")
            }
            BackgroundShellReadyWaitOutcome::BecameReady { waited_ms } => format!(
                "Service background shell job {job_label} became ready after {waited_ms}ms."
            ),
        };
        Ok(format!("{message}\nReady pattern: {ready_pattern}"))
    }

    #[cfg(test)]
    pub(crate) fn invoke_recipe_for_operator(
        &self,
        job_id: &str,
        recipe_name: &str,
    ) -> Result<String, String> {
        self.invoke_recipe(job_id, recipe_name, &HashMap::new(), None)
    }

    pub(crate) fn invoke_recipe_for_operator_with_args(
        &self,
        job_id: &str,
        recipe_name: &str,
        args: &HashMap<String, String>,
    ) -> Result<String, String> {
        self.invoke_recipe(job_id, recipe_name, args, None)
    }

    pub(crate) fn running_count(&self) -> usize {
        self.snapshots()
            .into_iter()
            .filter(|job| job.exit_code.is_none() && job.status == "running")
            .count()
    }

    pub(crate) fn running_count_by_intent(&self, intent: BackgroundShellIntent) -> usize {
        self.snapshots()
            .into_iter()
            .filter(|job| {
                job.exit_code.is_none() && job.status == "running" && job.intent == intent
            })
            .count()
    }

    pub(crate) fn running_service_count_by_readiness(
        &self,
        readiness: BackgroundShellServiceReadiness,
    ) -> usize {
        self.snapshots()
            .into_iter()
            .filter(|job| {
                job.exit_code.is_none()
                    && job.status == "running"
                    && job.intent == BackgroundShellIntent::Service
                    && job.service_readiness == Some(readiness)
            })
            .count()
    }

    pub(crate) fn service_capability_conflicts(&self) -> Vec<(String, Vec<String>)> {
        let mut conflicts = self
            .service_capability_index()
            .into_iter()
            .filter_map(|(capability, mut jobs)| {
                if jobs.len() < 2 {
                    None
                } else {
                    jobs.sort();
                    Some((capability, jobs))
                }
            })
            .collect::<Vec<_>>();
        conflicts.sort_by(|left, right| left.0.cmp(&right.0));
        conflicts
    }

    pub(crate) fn unique_service_capability_count(&self) -> usize {
        self.service_capability_index().len()
    }

    pub(crate) fn service_capability_index(&self) -> Vec<(String, Vec<String>)> {
        let mut index = BTreeMap::<String, Vec<String>>::new();
        for snapshot in self.running_service_snapshots() {
            let job_ref = snapshot
                .alias
                .as_deref()
                .map(|alias| format!("{} ({alias})", snapshot.id))
                .unwrap_or_else(|| snapshot.id.clone());
            for capability in snapshot.service_capabilities {
                index.entry(capability).or_default().push(job_ref.clone());
            }
        }
        index.into_iter().collect()
    }

    pub(crate) fn capability_dependency_summaries(
        &self,
    ) -> Vec<BackgroundShellCapabilityDependencySummary> {
        let services = self.running_service_snapshots();
        let mut summaries = Vec::new();
        for snapshot in self
            .snapshots()
            .into_iter()
            .filter(|job| job.exit_code.is_none() && job.status == "running")
        {
            for capability in &snapshot.dependency_capabilities {
                let providers = services
                    .iter()
                    .filter(|service| {
                        service
                            .service_capabilities
                            .iter()
                            .any(|entry| entry == capability)
                    })
                    .map(|service| {
                        service
                            .alias
                            .as_deref()
                            .map(|alias| format!("{} ({alias})", service.id))
                            .unwrap_or_else(|| service.id.clone())
                    })
                    .collect::<Vec<_>>();
                let status = if providers.is_empty() {
                    BackgroundShellCapabilityDependencyState::Missing
                } else if providers.len() > 1 {
                    BackgroundShellCapabilityDependencyState::Ambiguous
                } else if services.iter().any(|service| {
                    service
                        .service_capabilities
                        .iter()
                        .any(|entry| entry == capability)
                        && service.service_readiness
                            == Some(BackgroundShellServiceReadiness::Booting)
                }) {
                    BackgroundShellCapabilityDependencyState::Booting
                } else {
                    BackgroundShellCapabilityDependencyState::Satisfied
                };
                summaries.push(BackgroundShellCapabilityDependencySummary {
                    job_id: snapshot.id.clone(),
                    job_alias: snapshot.alias.clone(),
                    job_label: snapshot.label.clone(),
                    capability: capability.clone(),
                    blocking: snapshot.intent.is_blocking(),
                    status,
                    providers,
                });
            }
        }
        summaries.sort_by(|left, right| {
            left.blocking
                .cmp(&right.blocking)
                .reverse()
                .then_with(|| left.job_id.cmp(&right.job_id))
                .then_with(|| left.capability.cmp(&right.capability))
        });
        summaries
    }

    pub(crate) fn blocking_capability_dependency_issues(
        &self,
    ) -> Vec<BackgroundShellCapabilityDependencySummary> {
        self.capability_dependency_summaries()
            .into_iter()
            .filter(|summary| {
                summary.blocking
                    && !matches!(
                        summary.status,
                        BackgroundShellCapabilityDependencyState::Satisfied
                    )
            })
            .collect()
    }

    pub(crate) fn capability_dependency_count_by_state(
        &self,
        status: BackgroundShellCapabilityDependencyState,
    ) -> usize {
        self.capability_dependency_summaries()
            .into_iter()
            .filter(|summary| summary.status == status)
            .count()
    }

    pub(crate) fn service_capability_conflict_count(&self) -> usize {
        self.service_capability_conflicts().len()
    }

    pub(crate) fn job_count(&self) -> usize {
        self.inner
            .jobs
            .lock()
            .expect("background shell jobs lock")
            .len()
    }

    pub(crate) fn snapshots(&self) -> Vec<BackgroundShellJobSnapshot> {
        let mut jobs = self
            .inner
            .jobs
            .lock()
            .expect("background shell jobs lock")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut snapshots = jobs
            .drain(..)
            .map(|job| snapshot_from_job(&job))
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| left.id.cmp(&right.id));
        snapshots
    }

    pub(crate) fn terminate_all_running(&self) -> usize {
        let job_ids = self
            .snapshots()
            .into_iter()
            .filter(|job| job.status == "running")
            .map(|job| job.id)
            .collect::<Vec<_>>();
        terminate_jobs(self, job_ids)
    }

    pub(crate) fn terminate_running_by_intent(&self, intent: BackgroundShellIntent) -> usize {
        let job_ids = self
            .snapshots()
            .into_iter()
            .filter(|job| job.status == "running" && job.intent == intent)
            .map(|job| job.id)
            .collect::<Vec<_>>();
        terminate_jobs(self, job_ids)
    }

    pub(crate) fn render_for_ps(&self) -> Option<Vec<String>> {
        self.render_for_ps_filtered(None)
    }

    pub(crate) fn render_for_ps_filtered(
        &self,
        intent_filter: Option<BackgroundShellIntent>,
    ) -> Option<Vec<String>> {
        let snapshots = self.snapshots();
        let snapshots = snapshots
            .into_iter()
            .filter(|snapshot| intent_filter.is_none_or(|intent| snapshot.intent == intent))
            .collect::<Vec<_>>();
        if snapshots.is_empty() {
            return None;
        }
        let mut lines = vec!["Local background shell jobs:".to_string()];
        for (index, snapshot) in snapshots.into_iter().enumerate() {
            lines.push(format!(
                "{:>2}. {}  [{}]",
                index + 1,
                snapshot.command,
                snapshot.status
            ));
            lines.push(format!("    job      {}", snapshot.id));
            lines.push(format!("    process  {}", snapshot.pid));
            lines.push(format!("    cwd      {}", snapshot.cwd));
            lines.push(format!("    intent   {}", snapshot.intent.as_str()));
            if let Some(label) = snapshot.label.as_deref() {
                lines.push(format!("    label    {label}"));
            }
            if let Some(alias) = snapshot.alias.as_deref() {
                lines.push(format!("    alias    {alias}"));
            }
            if !snapshot.service_capabilities.is_empty() {
                lines.push(format!(
                    "    caps     {}",
                    snapshot.service_capabilities.join(", ")
                ));
            }
            if !snapshot.dependency_capabilities.is_empty() {
                lines.push(format!(
                    "    depends  {}",
                    snapshot
                        .dependency_capabilities
                        .iter()
                        .map(|capability| format!("@{capability}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if let Some(protocol) = snapshot.service_protocol.as_deref() {
                lines.push(format!("    protocol {protocol}"));
            }
            if let Some(endpoint) = snapshot.service_endpoint.as_deref() {
                lines.push(format!("    endpoint {endpoint}"));
            }
            if let Some(attach_hint) = snapshot.attach_hint.as_deref() {
                lines.push(format!("    attach   {attach_hint}"));
            }
            if !snapshot.interaction_recipes.is_empty() {
                lines.push(format!(
                    "    recipes  {}",
                    snapshot.interaction_recipes.len()
                ));
            }
            if let Some(ready_pattern) = snapshot.ready_pattern.as_deref() {
                lines.push(format!("    ready on {ready_pattern}"));
            }
            if let Some(service_readiness) = snapshot.service_readiness {
                lines.push(format!("    service  {}", service_readiness.as_str()));
            }
            lines.push(format!("    lines    {}", snapshot.total_lines));
            if let Some(source_thread_id) = snapshot.origin.source_thread_id.as_deref() {
                lines.push(format!("    origin   thread={source_thread_id}"));
            }
            if let Some(source_call_id) = snapshot.origin.source_call_id.as_deref() {
                lines.push(format!("    call     {source_call_id}"));
            }
            if !snapshot.recent_lines.is_empty() {
                lines.push(format!(
                    "    output   {}",
                    snapshot.recent_lines.join(" | ")
                ));
            }
        }
        if matches!(intent_filter, None | Some(BackgroundShellIntent::Service)) {
            if let Some(capability_lines) = self.render_service_capability_index_lines() {
                lines.extend(capability_lines);
            }
            let conflicts = self.service_capability_conflicts();
            if !conflicts.is_empty() {
                lines.push("Capability conflicts:".to_string());
                for (capability, jobs) in conflicts {
                    lines.push(format!("    @{capability} -> {}", jobs.join(", ")));
                }
            }
        }
        Some(lines)
    }

    pub(crate) fn render_service_capabilities_for_ps(&self) -> Option<Vec<String>> {
        let capability_index = self.service_capability_index();
        let mut consumer_index = BTreeMap::<String, Vec<String>>::new();
        for dependency in self.capability_dependency_summaries() {
            let consumer = dependency_consumer_display(&dependency);
            consumer_index
                .entry(dependency.capability)
                .or_default()
                .push(format!("{consumer} [{}]", dependency.status.as_str()));
        }
        let mut capabilities = capability_index
            .iter()
            .map(|(capability, _)| capability.clone())
            .collect::<BTreeSet<_>>();
        capabilities.extend(consumer_index.keys().cloned());
        if capabilities.is_empty() {
            return None;
        }
        let mut lines = vec!["Service capability index:".to_string()];
        for (index, capability) in capabilities.iter().enumerate() {
            let jobs = capability_index
                .iter()
                .find(|(entry, _)| entry == capability)
                .map(|(_, jobs)| jobs.clone())
                .unwrap_or_default();
            let qualifier = if jobs.len() > 1 { "  [conflict]" } else { "" };
            lines.push(format!(
                "{:>2}. @{} -> {}{}",
                index + 1,
                capability,
                if jobs.is_empty() {
                    "<missing provider>".to_string()
                } else {
                    jobs.join(", ")
                },
                qualifier
            ));
            if let Some(consumers) = consumer_index.get(capability) {
                lines.push(format!("    used by {}", consumers.join(", ")));
            }
        }
        lines.push(
            "Use @capability with :ps poll|send|attach|wait|run|terminate to target a reusable service."
                .to_string(),
        );
        Some(lines)
    }

    pub(crate) fn render_single_service_capability_for_ps(
        &self,
        capability_ref: &str,
    ) -> Result<Vec<String>, String> {
        let capability = if let Some(raw) = capability_ref.strip_prefix('@') {
            validate_service_capability(raw)?
        } else {
            validate_service_capability(capability_ref)?
        };
        let providers = self.running_service_providers_for_capability(&capability);
        let consumers = self
            .capability_dependency_summaries()
            .into_iter()
            .filter(|summary| summary.capability == capability)
            .collect::<Vec<_>>();
        if providers.is_empty() && consumers.is_empty() {
            return Err(format!("unknown service capability `@{capability}`"));
        }
        let mut lines = vec![format!("Service capability: @{capability}")];
        if providers.is_empty() {
            lines.push("Providers: <missing provider>".to_string());
        } else {
            lines.push("Providers:".to_string());
            for (index, provider) in providers.iter().enumerate() {
                lines.push(format!(
                    "{:>2}. {}  [{}]",
                    index + 1,
                    provider_display(provider),
                    provider
                        .service_readiness
                        .map(BackgroundShellServiceReadiness::as_str)
                        .unwrap_or("-")
                ));
                if let Some(protocol) = provider.service_protocol.as_deref() {
                    lines.push(format!("    protocol {protocol}"));
                }
                if let Some(endpoint) = provider.service_endpoint.as_deref() {
                    lines.push(format!("    endpoint {endpoint}"));
                }
                if !provider.interaction_recipes.is_empty() {
                    lines.push(format!(
                        "    recipes  {}",
                        provider.interaction_recipes.len()
                    ));
                }
            }
            if providers.len() > 1 {
                lines.push("Conflict: ambiguous capability provider set".to_string());
            }
        }
        if consumers.is_empty() {
            lines.push("Consumers: none".to_string());
        } else {
            lines.push("Consumers:".to_string());
            for (index, consumer) in consumers.iter().enumerate() {
                let job_ref = dependency_consumer_display(consumer);
                lines.push(format!(
                    "{:>2}. {}  [{}]  blocking={}",
                    index + 1,
                    job_ref,
                    consumer.status.as_str(),
                    if consumer.blocking { "yes" } else { "no" }
                ));
            }
        }
        Ok(lines)
    }

    fn lookup_job(&self, job_id: &str) -> Result<Arc<Mutex<BackgroundShellJobState>>, String> {
        self.inner
            .jobs
            .lock()
            .expect("background shell jobs lock")
            .get(job_id)
            .cloned()
            .ok_or_else(|| format!("unknown background shell job `{job_id}`"))
    }

    fn terminate_job(&self, job_id: &str) -> Result<(), String> {
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

    fn send_input_to_job(
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

    fn running_service_snapshots(&self) -> Vec<BackgroundShellJobSnapshot> {
        self.snapshots()
            .into_iter()
            .filter(|job| {
                job.intent == BackgroundShellIntent::Service
                    && job.exit_code.is_none()
                    && job.status == "running"
            })
            .collect()
    }

    fn running_service_providers_for_capability(
        &self,
        capability: &str,
    ) -> Vec<BackgroundShellJobSnapshot> {
        self.running_service_snapshots()
            .into_iter()
            .filter(|job| {
                job.service_capabilities
                    .iter()
                    .any(|entry| entry == capability)
            })
            .collect()
    }

    fn render_service_capability_index_lines(&self) -> Option<Vec<String>> {
        let capability_index = self.service_capability_index();
        let mut consumer_index = BTreeMap::<String, Vec<String>>::new();
        for dependency in self.capability_dependency_summaries() {
            let consumer = dependency
                .job_alias
                .as_deref()
                .map(|alias| format!("{} ({alias})", dependency.job_id))
                .unwrap_or_else(|| dependency.job_id.clone());
            consumer_index
                .entry(dependency.capability)
                .or_default()
                .push(format!("{consumer} [{}]", dependency.status.as_str()));
        }
        let mut capabilities = capability_index
            .iter()
            .map(|(capability, _)| capability.clone())
            .collect::<BTreeSet<_>>();
        capabilities.extend(consumer_index.keys().cloned());
        if capabilities.is_empty() {
            return None;
        }
        let mut lines = vec!["Capability index:".to_string()];
        for capability in capabilities {
            let jobs = capability_index
                .iter()
                .find(|(entry, _)| *entry == capability)
                .map(|(_, jobs)| jobs.clone())
                .unwrap_or_default();
            lines.push(format!(
                "    @{capability} -> {}",
                if jobs.is_empty() {
                    "<missing provider>".to_string()
                } else {
                    jobs.join(", ")
                }
            ));
            if let Some(consumers) = consumer_index.get(&capability) {
                lines.push(format!("      used by {}", consumers.join(", ")));
            }
        }
        Some(lines)
    }

    fn service_attachment_summary(&self, job_id: &str) -> Result<String, String> {
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

    fn invoke_recipe(
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

    fn wait_for_service_ready(
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
}

fn dependency_consumer_display(summary: &BackgroundShellCapabilityDependencySummary) -> String {
    if let Some(alias) = summary.job_alias.as_deref() {
        format!("{} ({alias})", summary.job_id)
    } else if let Some(label) = summary.job_label.as_deref() {
        format!("{} ({label})", summary.job_id)
    } else {
        summary.job_id.clone()
    }
}

fn provider_display(snapshot: &BackgroundShellJobSnapshot) -> String {
    if let Some(alias) = snapshot.alias.as_deref() {
        format!("{} ({alias})", snapshot.id)
    } else if let Some(label) = snapshot.label.as_deref() {
        format!("{} ({label})", snapshot.id)
    } else {
        snapshot.id.clone()
    }
}

fn resolve_background_cwd(raw_cwd: Option<&str>, resolved_cwd: &str) -> Result<PathBuf, String> {
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

fn parse_background_shell_intent(
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

fn parse_background_shell_label(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_background_shell_optional_string(
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

fn parse_background_shell_ready_pattern(
    value: Option<&serde_json::Value>,
) -> Result<Option<String>, String> {
    parse_background_shell_optional_string(value, "readyPattern")
}

fn parse_background_shell_capabilities(
    value: Option<&serde_json::Value>,
) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| "background_shell_start `capabilities` must be an array".to_string())?;
    let mut capabilities = Vec::with_capacity(array.len());
    for (index, item) in array.iter().enumerate() {
        let raw = item.as_str().ok_or_else(|| {
            format!("background_shell_start `capabilities[{index}]` must be a string")
        })?;
        capabilities.push(validate_service_capability(raw)?);
    }
    capabilities.sort();
    capabilities.dedup();
    Ok(capabilities)
}

fn parse_background_shell_timeout_ms(
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

fn validate_alias(alias: &str) -> Result<String, String> {
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

fn validate_service_capability(capability: &str) -> Result<String, String> {
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

impl BackgroundShellManager {
    fn resolve_service_capability_reference(&self, capability: &str) -> Result<String, String> {
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
                "unknown running background shell capability `@{capability}`; use /ps capabilities to inspect reusable service roles"
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
                    "background shell capability `@{capability}` is ambiguous across multiple running service jobs: {refs}; use /ps capabilities to inspect reusable service roles"
                ))
            }
        }
    }
}

fn spawn_shell_process(command: &str, cwd: &Path) -> Result<std::process::Child, String> {
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

fn spawn_output_reader<R>(
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

fn terminate_jobs(manager: &BackgroundShellManager, job_ids: Vec<String>) -> usize {
    let mut terminated = 0;
    for job_id in job_ids {
        if manager.terminate_job(&job_id).is_ok() {
            terminated += 1;
        }
    }
    terminated
}

fn snapshot_from_job(job: &Arc<Mutex<BackgroundShellJobState>>) -> BackgroundShellJobSnapshot {
    let state = job.lock().expect("background shell job lock");
    BackgroundShellJobSnapshot {
        id: state.id.clone(),
        pid: state.pid,
        command: state.command.clone(),
        cwd: state.cwd.clone(),
        intent: state.intent,
        label: state.label.clone(),
        alias: state.alias.clone(),
        service_capabilities: state.service_capabilities.clone(),
        dependency_capabilities: state.dependency_capabilities.clone(),
        service_protocol: state.service_protocol.clone(),
        service_endpoint: state.service_endpoint.clone(),
        attach_hint: state.attach_hint.clone(),
        interaction_recipes: state.interaction_recipes.clone(),
        ready_pattern: state.ready_pattern.clone(),
        service_readiness: service_readiness_for_state(&state),
        origin: state.origin.clone(),
        status: status_label(&state.status).to_string(),
        exit_code: exit_code(&state.status),
        total_lines: state.total_lines,
        recent_lines: state
            .lines
            .iter()
            .rev()
            .take(MAX_RENDERED_RECENT_LINES)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|line| summarize_line(&line.text))
            .collect(),
    }
}

fn service_readiness_for_state(
    state: &BackgroundShellJobState,
) -> Option<BackgroundShellServiceReadiness> {
    if state.intent != BackgroundShellIntent::Service {
        return None;
    }
    Some(match state.ready_pattern.as_deref() {
        Some(_) if state.service_ready => BackgroundShellServiceReadiness::Ready,
        Some(_) => BackgroundShellServiceReadiness::Booting,
        None => BackgroundShellServiceReadiness::Untracked,
    })
}

fn summarize_line(line: &str) -> String {
    const MAX_CHARS: usize = 120;
    let mut chars = line.chars();
    let summary = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{summary}...")
    } else {
        summary
    }
}

fn status_label(status: &BackgroundShellJobStatus) -> &str {
    match status {
        BackgroundShellJobStatus::Running => "running",
        BackgroundShellJobStatus::Completed(_) => "completed",
        BackgroundShellJobStatus::Failed(_) => "failed",
        BackgroundShellJobStatus::Terminated(_) => "terminated",
    }
}

fn exit_code(status: &BackgroundShellJobStatus) -> Option<i32> {
    match status {
        BackgroundShellJobStatus::Completed(code) => Some(*code),
        BackgroundShellJobStatus::Terminated(code) => *code,
        BackgroundShellJobStatus::Failed(_) | BackgroundShellJobStatus::Running => None,
    }
}

#[cfg(unix)]
fn terminate_pid(pid: u32) -> Result<(), String> {
    let status = Command::new("/bin/kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .map_err(|err| format!("failed to invoke kill for pid {pid}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("kill returned non-zero status for pid {pid}"))
    }
}

#[cfg(windows)]
fn terminate_pid(pid: u32) -> Result<(), String> {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .map_err(|err| format!("failed to invoke taskkill for pid {pid}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("taskkill returned non-zero status for pid {pid}"))
    }
}
