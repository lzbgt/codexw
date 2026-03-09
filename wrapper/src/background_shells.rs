use std::collections::HashMap;
use std::collections::VecDeque;
use std::process::ChildStdin;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;

#[path = "background_shells/execution.rs"]
mod execution;
#[path = "background_shells/recipes.rs"]
mod recipes;
#[path = "background_shells/services.rs"]
mod services;

#[cfg(test)]
#[path = "background_shells/tests.rs"]
mod tests;

pub(crate) use self::execution::parse_background_shell_optional_string;
pub(crate) use self::execution::terminate_jobs;
pub(crate) use self::execution::validate_service_capability;
use self::recipes::apply_recipe_arguments_to_action;
use self::recipes::interaction_action_summary;
use self::recipes::invoke_http_recipe;
use self::recipes::invoke_redis_recipe;
use self::recipes::invoke_tcp_recipe;
pub(crate) use self::recipes::parse_background_shell_interaction_recipes;
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
pub(crate) enum BackgroundShellReadyWaitOutcome {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundShellCapabilityIssueClass {
    Healthy,
    Missing,
    Booting,
    Untracked,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundShellServiceIssueClass {
    Ready,
    Booting,
    Untracked,
    Conflicts,
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

    fn lookup_job(&self, job_id: &str) -> Result<Arc<Mutex<BackgroundShellJobState>>, String> {
        self.inner
            .jobs
            .lock()
            .expect("background shell jobs lock")
            .get(job_id)
            .cloned()
            .ok_or_else(|| format!("unknown background shell job `{job_id}`"))
    }
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
