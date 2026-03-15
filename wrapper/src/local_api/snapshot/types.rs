use std::collections::BTreeMap;

use serde::Serialize;

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
    pub(crate) request_id: String,
    pub(crate) thread_name: String,
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
    pub(crate) recommended_action: String,
    pub(crate) recovery_policy: LocalApiRecoveryPolicy,
    pub(crate) recovery_options: Vec<LocalApiRecoveryOption>,
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
    pub(crate) request_id: String,
    pub(crate) thread_name: String,
    pub(crate) owner: String,
    pub(crate) source_call_id: Option<String>,
    pub(crate) target_background_shell_reference: Option<String>,
    pub(crate) target_background_shell_job_id: Option<String>,
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) observation_state: String,
    pub(crate) output_state: String,
    pub(crate) observed_background_shell_job: Option<LocalApiObservedBackgroundShellJob>,
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
