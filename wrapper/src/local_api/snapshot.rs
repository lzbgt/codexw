use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::app::build_resume_command;
use crate::app::current_program_name;
use crate::background_shells::BackgroundShellCapabilityIssueClass;
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellJobSnapshot;
use crate::orchestration_registry::active_sidecar_agent_task_count;
use crate::orchestration_registry::active_wait_task_count;
use crate::orchestration_registry::blocking_dependency_count;
use crate::orchestration_registry::main_agent_state_label;
use crate::orchestration_registry::orchestration_dependency_edges;
use crate::orchestration_registry::running_service_count_by_readiness;
use crate::orchestration_registry::running_shell_count_by_intent;
use crate::orchestration_registry::sidecar_dependency_count;
use crate::orchestration_registry::wait_dependency_summary;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LocalApiSnapshot {
    pub(crate) session_id: String,
    pub(crate) cwd: String,
    pub(crate) attachment_client_id: Option<String>,
    pub(crate) attachment_lease_seconds: Option<u64>,
    pub(crate) attachment_lease_expires_at_ms: Option<u64>,
    pub(crate) thread_id: Option<String>,
    pub(crate) active_turn_id: Option<String>,
    pub(crate) objective: Option<String>,
    pub(crate) turn_running: bool,
    pub(crate) started_turn_count: u64,
    pub(crate) completed_turn_count: u64,
    pub(crate) active_personality: Option<String>,
    pub(crate) async_tool_supervision: Option<LocalApiAsyncToolSupervision>,
    pub(crate) async_tool_backpressure: Option<LocalApiAsyncToolBackpressure>,
    pub(crate) async_tool_workers: Vec<LocalApiAsyncToolWorker>,
    pub(crate) supervision_notice: Option<LocalApiSupervisionNotice>,
    pub(crate) orchestration_status: LocalApiOrchestrationStatus,
    pub(crate) orchestration_dependencies: Vec<LocalApiDependencyEdge>,
    pub(crate) workers: LocalApiWorkersSnapshot,
    pub(crate) capabilities: Vec<LocalApiCapabilityEntry>,
    pub(crate) transcript: Vec<LocalApiTranscriptEntry>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LocalApiAsyncToolSupervision {
    pub(crate) classification: String,
    pub(crate) recommended_action: String,
    pub(crate) recovery_policy: LocalApiRecoveryPolicy,
    pub(crate) recovery_options: Vec<LocalApiRecoveryOption>,
    pub(crate) owner: String,
    pub(crate) source_call_id: Option<String>,
    pub(crate) target_background_shell_reference: Option<String>,
    pub(crate) target_background_shell_job_id: Option<String>,
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) observation_state: String,
    pub(crate) output_state: String,
    pub(crate) observed_background_shell_job: Option<LocalApiObservedBackgroundShellJob>,
    pub(crate) next_check_in_seconds: u64,
    pub(crate) elapsed_seconds: u64,
    pub(crate) active_request_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LocalApiAsyncToolBackpressure {
    pub(crate) abandoned_request_count: usize,
    pub(crate) saturation_threshold: usize,
    pub(crate) saturated: bool,
    pub(crate) oldest_request_id: String,
    pub(crate) oldest_thread_name: String,
    pub(crate) oldest_tool: String,
    pub(crate) oldest_summary: String,
    pub(crate) oldest_source_call_id: Option<String>,
    pub(crate) oldest_target_background_shell_reference: Option<String>,
    pub(crate) oldest_target_background_shell_job_id: Option<String>,
    pub(crate) oldest_observation_state: String,
    pub(crate) oldest_output_state: String,
    pub(crate) oldest_observed_background_shell_job: Option<LocalApiObservedBackgroundShellJob>,
    pub(crate) oldest_elapsed_before_timeout_seconds: u64,
    pub(crate) oldest_hard_timeout_seconds: u64,
    pub(crate) oldest_elapsed_seconds: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LocalApiAsyncToolWorker {
    pub(crate) request_id: String,
    pub(crate) lifecycle_state: String,
    pub(crate) thread_name: String,
    pub(crate) owner: String,
    pub(crate) source_call_id: Option<String>,
    pub(crate) target_background_shell_reference: Option<String>,
    pub(crate) target_background_shell_job_id: Option<String>,
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) observation_state: Option<String>,
    pub(crate) output_state: Option<String>,
    pub(crate) observed_background_shell_job: Option<LocalApiObservedBackgroundShellJob>,
    pub(crate) next_check_in_seconds: Option<u64>,
    pub(crate) runtime_elapsed_seconds: u64,
    pub(crate) state_elapsed_seconds: u64,
    pub(crate) hard_timeout_seconds: u64,
    pub(crate) supervision_classification: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LocalApiSupervisionNotice {
    pub(crate) classification: String,
    pub(crate) recommended_action: String,
    pub(crate) recovery_policy: LocalApiRecoveryPolicy,
    pub(crate) recovery_options: Vec<LocalApiRecoveryOption>,
    pub(crate) tool: String,
    pub(crate) summary: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LocalApiObservedBackgroundShellJob {
    pub(crate) job_id: String,
    pub(crate) status: String,
    pub(crate) command: String,
    pub(crate) total_lines: u64,
    pub(crate) last_output_age_seconds: Option<u64>,
    pub(crate) recent_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LocalApiRecoveryPolicy {
    pub(crate) kind: String,
    pub(crate) automation_ready: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LocalApiRecoveryOption {
    pub(crate) kind: String,
    pub(crate) label: String,
    pub(crate) automation_ready: bool,
    pub(crate) cli_command: Option<String>,
    pub(crate) local_api_method: Option<String>,
    pub(crate) local_api_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiOrchestrationStatus {
    pub(crate) main_agent_state: String,
    pub(crate) wait_summary: Option<String>,
    pub(crate) blocking_dependencies: usize,
    pub(crate) sidecar_dependencies: usize,
    pub(crate) wait_tasks: usize,
    pub(crate) sidecar_agent_tasks: usize,
    pub(crate) exec_prerequisites: usize,
    pub(crate) exec_sidecars: usize,
    pub(crate) exec_services: usize,
    pub(crate) services_ready: usize,
    pub(crate) services_booting: usize,
    pub(crate) services_untracked: usize,
    pub(crate) services_conflicted: usize,
    pub(crate) service_capabilities: usize,
    pub(crate) service_capability_conflicts: usize,
    pub(crate) capability_dependencies_missing: usize,
    pub(crate) capability_dependencies_booting: usize,
    pub(crate) capability_dependencies_ambiguous: usize,
    pub(crate) live_agent_task_count: usize,
    pub(crate) cached_agent_thread_count: usize,
    pub(crate) background_shell_job_count: usize,
    pub(crate) background_terminal_count: usize,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiDependencyEdge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) kind: String,
    pub(crate) blocking: bool,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiWorkersSnapshot {
    pub(crate) main_agent_state: String,
    pub(crate) wait_summary: Option<String>,
    pub(crate) cached_agent_threads: Vec<LocalApiCachedAgentThread>,
    pub(crate) live_agent_tasks: Vec<LocalApiLiveAgentTask>,
    pub(crate) background_shells: Vec<LocalApiBackgroundShellJob>,
    pub(crate) background_terminals: Vec<LocalApiBackgroundTerminal>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiCachedAgentThread {
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) preview: String,
    pub(crate) updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiLiveAgentTask {
    pub(crate) id: String,
    pub(crate) tool: String,
    pub(crate) status: String,
    pub(crate) sender_thread_id: String,
    pub(crate) receiver_thread_ids: Vec<String>,
    pub(crate) prompt: Option<String>,
    pub(crate) agent_statuses: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiBackgroundShellOrigin {
    pub(crate) source_thread_id: Option<String>,
    pub(crate) source_call_id: Option<String>,
    pub(crate) source_tool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiBackgroundShellJob {
    pub(crate) id: String,
    pub(crate) pid: u32,
    pub(crate) command: String,
    pub(crate) cwd: String,
    pub(crate) intent: String,
    pub(crate) label: Option<String>,
    pub(crate) alias: Option<String>,
    pub(crate) service_capabilities: Vec<String>,
    pub(crate) dependency_capabilities: Vec<String>,
    pub(crate) service_protocol: Option<String>,
    pub(crate) service_endpoint: Option<String>,
    pub(crate) attach_hint: Option<String>,
    pub(crate) interaction_recipe_names: Vec<String>,
    pub(crate) ready_pattern: Option<String>,
    pub(crate) service_readiness: Option<String>,
    pub(crate) origin: LocalApiBackgroundShellOrigin,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) total_lines: u64,
    pub(crate) last_output_age_seconds: Option<u64>,
    pub(crate) recent_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiBackgroundTerminal {
    pub(crate) item_id: String,
    pub(crate) process_id: String,
    pub(crate) command_display: String,
    pub(crate) waiting: bool,
    pub(crate) recent_inputs: Vec<String>,
    pub(crate) recent_output: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiCapabilityEntry {
    pub(crate) capability: String,
    pub(crate) issue: String,
    pub(crate) providers: Vec<LocalApiCapabilityProvider>,
    pub(crate) consumers: Vec<LocalApiCapabilityConsumer>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiCapabilityProvider {
    pub(crate) job_id: String,
    pub(crate) alias: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) readiness: Option<String>,
    pub(crate) protocol: Option<String>,
    pub(crate) endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiCapabilityConsumer {
    pub(crate) job_id: String,
    pub(crate) alias: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) blocking: bool,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub(crate) struct LocalApiTranscriptEntry {
    pub(crate) role: String,
    pub(crate) text: String,
}

pub(crate) type SharedSnapshot = Arc<RwLock<LocalApiSnapshot>>;

pub(crate) fn new_process_session_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    format!("sess_{:x}_{:x}", std::process::id(), millis)
}

pub(crate) fn new_shared_snapshot(session_id: String, cwd: String) -> SharedSnapshot {
    Arc::new(RwLock::new(LocalApiSnapshot {
        session_id,
        cwd,
        attachment_client_id: None,
        attachment_lease_seconds: None,
        attachment_lease_expires_at_ms: None,
        thread_id: None,
        active_turn_id: None,
        objective: None,
        turn_running: false,
        started_turn_count: 0,
        completed_turn_count: 0,
        active_personality: None,
        async_tool_supervision: None,
        async_tool_backpressure: None,
        async_tool_workers: Vec::new(),
        supervision_notice: None,
        orchestration_status: LocalApiOrchestrationStatus::default(),
        orchestration_dependencies: Vec::new(),
        workers: LocalApiWorkersSnapshot::default(),
        capabilities: Vec::new(),
        transcript: Vec::new(),
    }))
}

pub(crate) fn sync_shared_snapshot(
    snapshot: &SharedSnapshot,
    state: &AppState,
) -> LocalApiSnapshot {
    if let Ok(mut guard) = snapshot.write() {
        guard.thread_id = state.thread_id.clone();
        guard.active_turn_id = state.active_turn_id.clone();
        guard.objective = state.objective.clone();
        guard.turn_running = state.turn_running;
        guard.started_turn_count = state.started_turn_count;
        guard.completed_turn_count = state.completed_turn_count;
        guard.active_personality = state.active_personality.clone();
        guard.async_tool_supervision =
            async_tool_supervision_snapshot(&guard.session_id, &guard.cwd, state);
        guard.async_tool_backpressure = async_tool_backpressure_snapshot(state);
        guard.async_tool_workers = async_tool_workers_snapshot(state);
        guard.supervision_notice =
            supervision_notice_snapshot(&guard.session_id, &guard.cwd, state);
        guard.orchestration_status = orchestration_status_snapshot(state);
        guard.orchestration_dependencies = orchestration_dependencies_snapshot(state);
        guard.workers = workers_snapshot(state);
        guard.capabilities = capabilities_snapshot(state);
        guard.transcript = transcript_snapshot(state);
        return guard.clone();
    }

    LocalApiSnapshot {
        session_id: String::new(),
        cwd: String::new(),
        attachment_client_id: None,
        attachment_lease_seconds: None,
        attachment_lease_expires_at_ms: None,
        thread_id: None,
        active_turn_id: None,
        objective: None,
        turn_running: false,
        started_turn_count: 0,
        completed_turn_count: 0,
        active_personality: None,
        async_tool_supervision: None,
        async_tool_backpressure: None,
        async_tool_workers: Vec::new(),
        supervision_notice: None,
        orchestration_status: LocalApiOrchestrationStatus::default(),
        orchestration_dependencies: Vec::new(),
        workers: LocalApiWorkersSnapshot::default(),
        capabilities: Vec::new(),
        transcript: Vec::new(),
    }
}

fn async_tool_supervision_snapshot(
    session_id: &str,
    cwd: &str,
    state: &AppState,
) -> Option<LocalApiAsyncToolSupervision> {
    let activity = state.oldest_async_tool_activity()?;
    let classification = activity.supervision_class()?;
    let observation = state.async_tool_observation(activity);
    Some(LocalApiAsyncToolSupervision {
        classification: classification.label().to_string(),
        recommended_action: classification.recommended_action().to_string(),
        recovery_policy: LocalApiRecoveryPolicy {
            kind: classification.recovery_policy_kind().label().to_string(),
            automation_ready: classification.automation_ready(),
        },
        recovery_options: recovery_options_snapshot(session_id, cwd, state, classification),
        owner: observation.owner_kind.label().to_string(),
        source_call_id: activity.source_call_id.clone(),
        target_background_shell_reference: activity.target_background_shell_reference.clone(),
        target_background_shell_job_id: activity.target_background_shell_job_id.clone(),
        tool: activity.tool.clone(),
        summary: activity.summary.clone(),
        observation_state: observation.observation_state.label().to_string(),
        output_state: observation.output_state.label().to_string(),
        observed_background_shell_job: observation
            .observed_background_shell_job
            .map(local_api_observed_background_shell_job),
        next_check_in_seconds: activity.next_health_check_in().as_secs(),
        elapsed_seconds: activity.elapsed().as_secs(),
        active_request_count: state.active_async_tool_requests.len(),
    })
}

fn async_tool_backpressure_snapshot(state: &AppState) -> Option<LocalApiAsyncToolBackpressure> {
    let (request_id, abandoned) = state.oldest_abandoned_async_tool_entry()?;
    let observation = state.abandoned_async_tool_observation(abandoned);
    Some(LocalApiAsyncToolBackpressure {
        abandoned_request_count: state.abandoned_async_tool_request_count(),
        saturation_threshold: crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS,
        saturated: state.async_tool_backpressure_active(),
        oldest_request_id: crate::state::request_id_label(request_id),
        oldest_thread_name: abandoned.worker_thread_name.clone(),
        oldest_tool: abandoned.tool.clone(),
        oldest_summary: abandoned.summary.clone(),
        oldest_source_call_id: abandoned.source_call_id.clone(),
        oldest_target_background_shell_reference: abandoned
            .target_background_shell_reference
            .clone(),
        oldest_target_background_shell_job_id: abandoned.target_background_shell_job_id.clone(),
        oldest_observation_state: observation.observation_state.label().to_string(),
        oldest_output_state: observation.output_state.label().to_string(),
        oldest_observed_background_shell_job: observation
            .observed_background_shell_job
            .map(local_api_observed_background_shell_job),
        oldest_elapsed_before_timeout_seconds: abandoned.elapsed_before_timeout.as_secs(),
        oldest_hard_timeout_seconds: abandoned.hard_timeout.as_secs(),
        oldest_elapsed_seconds: abandoned.timed_out_elapsed().as_secs(),
    })
}

fn async_tool_workers_snapshot(state: &AppState) -> Vec<LocalApiAsyncToolWorker> {
    state
        .async_tool_worker_statuses()
        .into_iter()
        .map(|worker| LocalApiAsyncToolWorker {
            request_id: worker.request_id,
            lifecycle_state: worker.lifecycle_state.label().to_string(),
            thread_name: worker.worker_thread_name,
            owner: worker.owner_kind.label().to_string(),
            source_call_id: worker.source_call_id,
            target_background_shell_reference: worker.target_background_shell_reference,
            target_background_shell_job_id: worker.target_background_shell_job_id,
            tool: worker.tool,
            summary: worker.summary,
            observation_state: worker
                .observation_state
                .map(|observation_state| observation_state.label().to_string()),
            output_state: worker
                .output_state
                .map(|output_state| output_state.label().to_string()),
            observed_background_shell_job: worker
                .observed_background_shell_job
                .map(local_api_observed_background_shell_job),
            next_check_in_seconds: worker.next_health_check_in.map(|value| value.as_secs()),
            runtime_elapsed_seconds: worker.runtime_elapsed.as_secs(),
            state_elapsed_seconds: worker.state_elapsed.as_secs(),
            hard_timeout_seconds: worker.hard_timeout.as_secs(),
            supervision_classification: worker
                .supervision_classification
                .map(|classification| classification.label().to_string()),
        })
        .collect()
}

fn supervision_notice_snapshot(
    session_id: &str,
    cwd: &str,
    state: &AppState,
) -> Option<LocalApiSupervisionNotice> {
    let notice = state
        .active_supervision_notice
        .clone()
        .or_else(|| state.current_async_tool_supervision_notice())?;
    Some(LocalApiSupervisionNotice {
        classification: notice.classification.label().to_string(),
        recommended_action: notice.recommended_action().to_string(),
        recovery_policy: LocalApiRecoveryPolicy {
            kind: notice.recovery_policy_kind().label().to_string(),
            automation_ready: notice.automation_ready(),
        },
        recovery_options: recovery_options_snapshot(session_id, cwd, state, notice.classification),
        tool: notice.tool.clone(),
        summary: notice.summary.clone(),
    })
}

fn local_api_observed_background_shell_job(
    job: crate::state::AsyncToolObservedBackgroundShellJob,
) -> LocalApiObservedBackgroundShellJob {
    LocalApiObservedBackgroundShellJob {
        job_id: job.job_id,
        status: job.status,
        command: job.command,
        total_lines: job.total_lines,
        last_output_age_seconds: job.last_output_age.map(|value| value.as_secs()),
        recent_lines: job.recent_lines,
    }
}

fn recovery_options_snapshot(
    session_id: &str,
    cwd: &str,
    state: &AppState,
    classification: crate::state::AsyncToolSupervisionClass,
) -> Vec<LocalApiRecoveryOption> {
    let mut options = Vec::new();
    match classification {
        crate::state::AsyncToolSupervisionClass::ToolSlow => {
            options.push(LocalApiRecoveryOption {
                kind: "observe_status".to_string(),
                label: "Observe current session status".to_string(),
                automation_ready: false,
                cli_command: None,
                local_api_method: Some("GET".to_string()),
                local_api_path: Some(format!("/api/v1/session/{session_id}")),
            });
            if state.turn_running || state.active_turn_id.is_some() {
                options.push(LocalApiRecoveryOption {
                    kind: "interrupt_turn".to_string(),
                    label: "Interrupt the active turn".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: Some(format!("/api/v1/session/{session_id}/turn/interrupt")),
                });
            }
        }
        crate::state::AsyncToolSupervisionClass::ToolWedged => {
            if state.turn_running || state.active_turn_id.is_some() {
                options.push(LocalApiRecoveryOption {
                    kind: "interrupt_turn".to_string(),
                    label: "Interrupt the active turn".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: Some(format!("/api/v1/session/{session_id}/turn/interrupt")),
                });
            }
            if let Some(thread_id) = state.thread_id.as_deref() {
                options.push(LocalApiRecoveryOption {
                    kind: "exit_and_resume".to_string(),
                    label: "Exit and resume the thread in a newer client".to_string(),
                    automation_ready: false,
                    cli_command: Some(build_resume_command(
                        &current_program_name(),
                        cwd,
                        thread_id,
                    )),
                    local_api_method: None,
                    local_api_path: None,
                });
            }
        }
    }
    options
}

fn orchestration_status_snapshot(state: &AppState) -> LocalApiOrchestrationStatus {
    LocalApiOrchestrationStatus {
        main_agent_state: main_agent_state_label(state).to_string(),
        wait_summary: wait_dependency_summary(state),
        blocking_dependencies: blocking_dependency_count(state),
        sidecar_dependencies: sidecar_dependency_count(state),
        wait_tasks: active_wait_task_count(state),
        sidecar_agent_tasks: active_sidecar_agent_task_count(state),
        exec_prerequisites: running_shell_count_by_intent(
            state,
            BackgroundShellIntent::Prerequisite,
        ),
        exec_sidecars: running_shell_count_by_intent(state, BackgroundShellIntent::Observation),
        exec_services: running_shell_count_by_intent(state, BackgroundShellIntent::Service),
        services_ready: running_service_count_by_readiness(
            state,
            crate::background_shells::BackgroundShellServiceReadiness::Ready,
        ),
        services_booting: running_service_count_by_readiness(
            state,
            crate::background_shells::BackgroundShellServiceReadiness::Booting,
        ),
        services_untracked: running_service_count_by_readiness(
            state,
            crate::background_shells::BackgroundShellServiceReadiness::Untracked,
        ),
        services_conflicted: state
            .orchestration
            .background_shells
            .service_conflicting_job_count(),
        service_capabilities: state
            .orchestration
            .background_shells
            .unique_service_capability_count(),
        service_capability_conflicts: state
            .orchestration
            .background_shells
            .service_capability_conflict_count(),
        capability_dependencies_missing: state
            .orchestration
            .background_shells
            .capability_dependency_count_by_state(
                crate::background_shells::BackgroundShellCapabilityDependencyState::Missing,
            ),
        capability_dependencies_booting: state
            .orchestration
            .background_shells
            .capability_dependency_count_by_state(
                crate::background_shells::BackgroundShellCapabilityDependencyState::Booting,
            ),
        capability_dependencies_ambiguous: state
            .orchestration
            .background_shells
            .capability_dependency_count_by_state(
                crate::background_shells::BackgroundShellCapabilityDependencyState::Ambiguous,
            ),
        live_agent_task_count: state.orchestration.live_agent_tasks.len(),
        cached_agent_thread_count: state.orchestration.cached_agent_threads.len(),
        background_shell_job_count: state.orchestration.background_shells.job_count(),
        background_terminal_count: state.orchestration.background_terminals.len(),
    }
}

fn orchestration_dependencies_snapshot(state: &AppState) -> Vec<LocalApiDependencyEdge> {
    orchestration_dependency_edges(state)
        .into_iter()
        .map(|edge| LocalApiDependencyEdge {
            from: edge.from,
            to: edge.to,
            kind: edge.kind,
            blocking: edge.blocking,
        })
        .collect()
}

fn workers_snapshot(state: &AppState) -> LocalApiWorkersSnapshot {
    let mut live_agent_tasks = state
        .orchestration
        .live_agent_tasks
        .values()
        .cloned()
        .collect::<Vec<_>>();
    live_agent_tasks.sort_by(|left, right| left.id.cmp(&right.id));

    let mut background_terminals = state
        .orchestration
        .background_terminals
        .values()
        .cloned()
        .collect::<Vec<_>>();
    background_terminals.sort_by(|left, right| left.process_id.cmp(&right.process_id));

    LocalApiWorkersSnapshot {
        main_agent_state: main_agent_state_label(state).to_string(),
        wait_summary: wait_dependency_summary(state),
        cached_agent_threads: state
            .orchestration
            .cached_agent_threads
            .iter()
            .cloned()
            .map(|thread| LocalApiCachedAgentThread {
                id: thread.id,
                status: thread.status,
                preview: thread.preview,
                updated_at: thread.updated_at,
            })
            .collect(),
        live_agent_tasks: live_agent_tasks
            .into_iter()
            .map(|task| LocalApiLiveAgentTask {
                id: task.id,
                tool: task.tool,
                status: task.status,
                sender_thread_id: task.sender_thread_id,
                receiver_thread_ids: task.receiver_thread_ids,
                prompt: task.prompt,
                agent_statuses: task.agent_statuses,
            })
            .collect(),
        background_shells: state
            .orchestration
            .background_shells
            .snapshots()
            .into_iter()
            .map(local_api_shell_job)
            .collect(),
        background_terminals: background_terminals
            .into_iter()
            .map(|terminal| LocalApiBackgroundTerminal {
                item_id: terminal.item_id,
                process_id: terminal.process_id,
                command_display: terminal.command_display,
                waiting: terminal.waiting,
                recent_inputs: terminal.recent_inputs,
                recent_output: terminal.recent_output,
            })
            .collect(),
    }
}

pub(crate) fn local_api_shell_job(
    snapshot: BackgroundShellJobSnapshot,
) -> LocalApiBackgroundShellJob {
    LocalApiBackgroundShellJob {
        id: snapshot.id,
        pid: snapshot.pid,
        command: snapshot.command,
        cwd: snapshot.cwd,
        intent: snapshot.intent.as_str().to_string(),
        label: snapshot.label,
        alias: snapshot.alias,
        service_capabilities: snapshot.service_capabilities,
        dependency_capabilities: snapshot.dependency_capabilities,
        service_protocol: snapshot.service_protocol,
        service_endpoint: snapshot.service_endpoint,
        attach_hint: snapshot.attach_hint,
        interaction_recipe_names: snapshot
            .interaction_recipes
            .into_iter()
            .map(|recipe| recipe.name)
            .collect(),
        ready_pattern: snapshot.ready_pattern,
        service_readiness: snapshot
            .service_readiness
            .map(|value| value.as_str().to_string()),
        origin: LocalApiBackgroundShellOrigin {
            source_thread_id: snapshot.origin.source_thread_id,
            source_call_id: snapshot.origin.source_call_id,
            source_tool: snapshot.origin.source_tool,
        },
        status: snapshot.status,
        exit_code: snapshot.exit_code,
        total_lines: snapshot.total_lines,
        last_output_age_seconds: snapshot.last_output_age.map(|value| value.as_secs()),
        recent_lines: snapshot.recent_lines,
    }
}

fn capabilities_snapshot(state: &AppState) -> Vec<LocalApiCapabilityEntry> {
    let manager = &state.orchestration.background_shells;
    let dependency_summaries = manager.capability_dependency_summaries();
    let mut capabilities = manager
        .service_capability_index()
        .into_iter()
        .map(|(capability, _)| capability)
        .collect::<std::collections::BTreeSet<_>>();
    capabilities.extend(
        dependency_summaries
            .iter()
            .map(|summary| summary.capability.clone()),
    );

    let mut entries = Vec::new();
    for capability in capabilities {
        let issue = manager
            .service_capability_issue_for_ref(&capability)
            .unwrap_or(BackgroundShellCapabilityIssueClass::Missing);
        let providers = manager
            .running_service_providers_for_capability(&capability)
            .into_iter()
            .map(|job| LocalApiCapabilityProvider {
                job_id: job.id,
                alias: job.alias,
                label: job.label,
                readiness: job
                    .service_readiness
                    .map(|value| value.as_str().to_string()),
                protocol: job.service_protocol,
                endpoint: job.service_endpoint,
            })
            .collect::<Vec<_>>();
        let consumers = dependency_summaries
            .iter()
            .filter(|summary| summary.capability == capability)
            .map(|summary| LocalApiCapabilityConsumer {
                job_id: summary.job_id.clone(),
                alias: summary.job_alias.clone(),
                label: summary.job_label.clone(),
                blocking: summary.blocking,
                status: summary.status.as_str().to_string(),
            })
            .collect::<Vec<_>>();
        entries.push(LocalApiCapabilityEntry {
            capability,
            issue: capability_issue_label(issue).to_string(),
            providers,
            consumers,
        });
    }
    entries.sort_by(|left, right| left.capability.cmp(&right.capability));
    entries
}

fn capability_issue_label(issue: BackgroundShellCapabilityIssueClass) -> &'static str {
    match issue {
        BackgroundShellCapabilityIssueClass::Healthy => "healthy",
        BackgroundShellCapabilityIssueClass::Missing => "missing",
        BackgroundShellCapabilityIssueClass::Booting => "booting",
        BackgroundShellCapabilityIssueClass::Untracked => "untracked",
        BackgroundShellCapabilityIssueClass::Ambiguous => "ambiguous",
    }
}

fn transcript_snapshot(state: &AppState) -> Vec<LocalApiTranscriptEntry> {
    state
        .conversation_history
        .iter()
        .cloned()
        .map(|message| LocalApiTranscriptEntry {
            role: message.role,
            text: message.text,
        })
        .collect()
}
