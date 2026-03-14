use std::sync::Arc;
use std::sync::Mutex;

use super::core::BackgroundShellIntent;
use super::core::BackgroundShellJobSnapshot;
use super::core::BackgroundShellJobState;
use super::core::BackgroundShellJobStatus;
use super::core::BackgroundShellManager;
use super::core::BackgroundShellServiceReadiness;

const MAX_RENDERED_RECENT_LINES: usize = 3;

impl BackgroundShellManager {
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
}

fn snapshot_from_job(job: &Arc<Mutex<BackgroundShellJobState>>) -> BackgroundShellJobSnapshot {
    let state = job.lock().expect("background shell job lock");
    let now = std::time::Instant::now();
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
        last_output_age: state
            .last_output_at
            .map(|last_output_at| now.saturating_duration_since(last_output_at)),
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

pub(crate) fn service_readiness_for_state(
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

pub(crate) fn status_label(status: &BackgroundShellJobStatus) -> &str {
    match status {
        BackgroundShellJobStatus::Running => "running",
        BackgroundShellJobStatus::Completed(code) if *code == 0 => "completed",
        BackgroundShellJobStatus::Completed(_) => "failed",
        BackgroundShellJobStatus::Failed(_) => "failed",
        BackgroundShellJobStatus::Terminated(_) => "terminated",
    }
}

pub(crate) fn exit_code(status: &BackgroundShellJobStatus) -> Option<i32> {
    match status {
        BackgroundShellJobStatus::Completed(code) => Some(*code),
        BackgroundShellJobStatus::Terminated(code) => *code,
        BackgroundShellJobStatus::Failed(_) | BackgroundShellJobStatus::Running => None,
    }
}
