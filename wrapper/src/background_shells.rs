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

const DEFAULT_POLL_LIMIT: usize = 40;
const MAX_POLL_LIMIT: usize = 200;
const MAX_STORED_LINES: usize = 2_000;
const MAX_RENDERED_RECENT_LINES: usize = 3;

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
        let ready_pattern = parse_background_shell_ready_pattern(object.get("readyPattern"))?;
        if ready_pattern.is_some() && intent != BackgroundShellIntent::Service {
            return Err(
                "background_shell_start `readyPattern` is only supported when `intent=service`"
                    .to_string(),
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

    pub(crate) fn resolve_job_reference(&self, reference: &str) -> Result<String, String> {
        let reference = reference.trim();
        if reference.is_empty() {
            return Err("background shell job reference cannot be empty".to_string());
        }
        if reference.starts_with("bg-") {
            self.lookup_job(reference)?;
            return Ok(reference.to_string());
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
        Some(lines)
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

fn parse_background_shell_ready_pattern(
    value: Option<&serde_json::Value>,
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
                "background_shell_start `readyPattern` must be a non-empty string".to_string()
            }),
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

#[cfg(test)]
mod tests {
    use super::BackgroundShellIntent;
    use super::BackgroundShellManager;
    use super::BackgroundShellOrigin;
    use super::BackgroundShellServiceReadiness;
    use serde_json::json;
    use std::thread;
    use std::time::Duration;

    #[cfg(unix)]
    fn interactive_echo_command() -> &'static str {
        "cat"
    }

    #[cfg(windows)]
    fn interactive_echo_command() -> &'static str {
        "more"
    }

    #[cfg(unix)]
    fn service_ready_command() -> &'static str {
        "printf 'booting\\nREADY\\n'; sleep 0.4"
    }

    #[cfg(windows)]
    fn service_ready_command() -> &'static str {
        "echo booting && echo READY && ping -n 2 127.0.0.1 >NUL"
    }

    #[test]
    fn background_shell_job_can_start_and_poll_output() {
        let manager = BackgroundShellManager::default();
        let started = manager
            .start_from_tool(&json!({"command": "printf 'alpha\\nbeta\\n'"}), "/tmp")
            .expect("start background shell");
        assert!(started.contains("Started background shell job bg-1"));

        let mut rendered = String::new();
        for _ in 0..20 {
            rendered = manager
                .poll_from_tool(&json!({"jobId": "bg-1"}))
                .expect("poll background shell");
            if rendered.contains("alpha") && rendered.contains("beta") {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }

        assert!(rendered.contains("Job: bg-1"));
        assert!(rendered.contains("alpha"));
        assert!(rendered.contains("beta"));
    }

    #[test]
    fn background_shell_job_accepts_stdin_and_emits_output() {
        let manager = BackgroundShellManager::default();
        manager
            .start_from_tool(&json!({"command": interactive_echo_command()}), "/tmp")
            .expect("start interactive background shell");

        manager
            .send_input_for_operator("bg-1", "hello from stdin", true)
            .expect("send stdin");

        let mut rendered = String::new();
        for _ in 0..40 {
            rendered = manager
                .poll_from_tool(&json!({"jobId": "bg-1"}))
                .expect("poll background shell");
            if rendered.contains("hello from stdin") {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }

        assert!(rendered.contains("hello from stdin"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_list_reports_running_jobs() {
        let manager = BackgroundShellManager::default();
        manager
            .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
            .expect("start background shell");
        let rendered = manager.list_from_tool();
        assert!(rendered.contains("Background shell jobs:"));
        assert!(rendered.contains("bg-1"));
        assert!(rendered.contains("running"));
        assert!(rendered.contains("intent=observation"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_origin_intent_and_label_are_preserved_in_snapshots_and_poll() {
        let manager = BackgroundShellManager::default();
        manager
            .start_from_tool_with_context(
                &json!({
                    "command": "sleep 0.4",
                    "intent": "service",
                    "label": "webpack dev server"
                }),
                "/tmp",
                BackgroundShellOrigin {
                    source_thread_id: Some("thread-agent-1".to_string()),
                    source_call_id: Some("call-77".to_string()),
                    source_tool: Some("background_shell_start".to_string()),
                },
            )
            .expect("start background shell");

        let snapshots = manager.snapshots();
        assert_eq!(
            snapshots[0].origin.source_thread_id.as_deref(),
            Some("thread-agent-1")
        );
        assert_eq!(snapshots[0].intent, BackgroundShellIntent::Service);
        assert_eq!(snapshots[0].label.as_deref(), Some("webpack dev server"));
        let rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll background shell");
        assert!(rendered.contains("Intent: service"));
        assert!(rendered.contains("Label: webpack dev server"));
        assert!(rendered.contains("Source thread: thread-agent-1"));
        assert!(rendered.contains("Source call: call-77"));
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn service_shell_ready_pattern_transitions_from_booting_to_ready() {
        let manager = BackgroundShellManager::default();
        manager
            .start_from_tool(
                &json!({
                    "command": service_ready_command(),
                    "intent": "service",
                    "readyPattern": "READY"
                }),
                "/tmp",
            )
            .expect("start ready-pattern service shell");

        let mut rendered = String::new();
        for _ in 0..40 {
            rendered = manager
                .poll_from_tool(&json!({"jobId": "bg-1"}))
                .expect("poll service shell");
            if rendered.contains("Service state: ready") {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }

        assert!(rendered.contains("Ready pattern: READY"));
        assert!(rendered.contains("Service state: ready"));
        assert_eq!(
            manager.running_service_count_by_readiness(BackgroundShellServiceReadiness::Ready),
            1
        );
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn ready_pattern_requires_service_intent() {
        let manager = BackgroundShellManager::default();
        let err = manager
            .start_from_tool(
                &json!({
                    "command": "sleep 0.1",
                    "intent": "observation",
                    "readyPattern": "READY"
                }),
                "/tmp",
            )
            .expect_err("readyPattern should require service intent");
        assert!(err.contains("readyPattern"));
        assert_eq!(manager.job_count(), 0);
    }

    #[test]
    fn background_shell_manager_counts_running_jobs_by_intent() {
        let manager = BackgroundShellManager::default();
        manager
            .start_from_tool(
                &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
                "/tmp",
            )
            .expect("start prerequisite background shell");
        manager
            .start_from_tool(
                &json!({"command": "sleep 0.4", "intent": "service"}),
                "/tmp",
            )
            .expect("start service background shell");
        manager
            .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
            .expect("start observation background shell");

        assert_eq!(
            manager.running_count_by_intent(BackgroundShellIntent::Prerequisite),
            1
        );
        assert_eq!(
            manager.running_count_by_intent(BackgroundShellIntent::Service),
            1
        );
        assert_eq!(
            manager.running_count_by_intent(BackgroundShellIntent::Observation),
            1
        );
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_manager_can_terminate_only_selected_intent() {
        let manager = BackgroundShellManager::default();
        manager
            .start_from_tool(
                &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
                "/tmp",
            )
            .expect("start prerequisite background shell");
        manager
            .start_from_tool(
                &json!({"command": "sleep 0.4", "intent": "service"}),
                "/tmp",
            )
            .expect("start service background shell");
        manager
            .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
            .expect("start observation background shell");

        assert_eq!(
            manager.terminate_running_by_intent(BackgroundShellIntent::Service),
            1
        );
        assert_eq!(
            manager.running_count_by_intent(BackgroundShellIntent::Service),
            0
        );
        assert_eq!(
            manager.running_count_by_intent(BackgroundShellIntent::Prerequisite),
            1
        );
        assert_eq!(
            manager.running_count_by_intent(BackgroundShellIntent::Observation),
            1
        );
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_manager_resolves_job_references_by_id_and_index() {
        let manager = BackgroundShellManager::default();
        manager
            .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
            .expect("start shell 1");
        manager
            .start_from_tool(
                &json!({"command": "sleep 0.4", "intent": "service"}),
                "/tmp",
            )
            .expect("start shell 2");

        assert_eq!(
            manager
                .resolve_job_reference("bg-1")
                .expect("resolve by id"),
            "bg-1"
        );
        assert_eq!(
            manager
                .resolve_job_reference("2")
                .expect("resolve by index"),
            "bg-2"
        );
        manager.set_job_alias("bg-2", "dev.api").expect("set alias");
        assert_eq!(
            manager
                .resolve_job_reference("dev.api")
                .expect("resolve by alias"),
            "bg-2"
        );
        assert!(manager.resolve_job_reference("0").is_err());
        assert!(manager.resolve_job_reference("bg-9").is_err());
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_manager_can_set_and_clear_aliases() {
        let manager = BackgroundShellManager::default();
        manager
            .start_from_tool(
                &json!({"command": "sleep 0.4", "intent": "service", "label": "dev server"}),
                "/tmp",
            )
            .expect("start shell");

        manager
            .set_job_alias("bg-1", "dev_server")
            .expect("set alias");
        let snapshots = manager.snapshots();
        assert_eq!(snapshots[0].alias.as_deref(), Some("dev_server"));
        let rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll background shell");
        assert!(rendered.contains("Alias: dev_server"));

        let cleared = manager.clear_job_alias("dev_server").expect("clear alias");
        assert_eq!(cleared, "bg-1");
        let snapshots = manager.snapshots();
        assert!(snapshots[0].alias.is_none());
        let _ = manager.terminate_all_running();
    }

    #[test]
    fn background_shell_send_from_tool_resolves_aliases() {
        let manager = BackgroundShellManager::default();
        manager
            .start_from_tool(&json!({"command": interactive_echo_command()}), "/tmp")
            .expect("start shell");
        manager.set_job_alias("bg-1", "dev.api").expect("set alias");

        let rendered = manager
            .send_input_from_tool(&json!({
                "jobId": "dev.api",
                "text": "ping via alias"
            }))
            .expect("send via alias");

        assert!(rendered.contains("Sent"));
        let _ = manager.terminate_all_running();
    }
}
