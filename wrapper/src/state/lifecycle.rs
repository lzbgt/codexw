use std::collections::HashMap;
use std::time::Duration;

use crate::rpc::RequestId;

use super::AbandonedAsyncToolRequest;
use super::AppState;
use super::AsyncToolObservation;
use super::AsyncToolObservationState;
use super::AsyncToolObservedBackgroundShellJob;
use super::AsyncToolOutputState;
use super::AsyncToolWorkerLifecycleState;
use super::AsyncToolWorkerStatus;
use super::ConversationMessage;
#[cfg(test)]
use super::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT;
use super::MAX_ABANDONED_ASYNC_TOOL_REQUESTS;
use super::OrchestrationState;
use super::SessionOverrides;
use super::TimedOutAsyncToolRequest;

impl OrchestrationState {
    pub(crate) fn reset_thread_context(&mut self) {
        self.background_terminals.clear();
        self.cached_agent_threads.clear();
        self.live_agent_tasks.clear();
    }
}

impl AppState {
    pub(crate) fn new(auto_continue: bool, raw_json: bool) -> Self {
        Self {
            thread_id: None,
            active_turn_id: None,
            active_exec_process_id: None,
            active_async_tool_requests: HashMap::new(),
            abandoned_async_tool_requests: HashMap::new(),
            realtime_active: false,
            realtime_session_id: None,
            realtime_last_error: None,
            realtime_started_at: None,
            realtime_prompt: None,
            pending_thread_switch: false,
            turn_running: false,
            activity_started_at: None,
            started_turn_count: 0,
            completed_turn_count: 0,
            auto_continue,
            startup_resume_picker: false,
            objective: None,
            last_agent_message: None,
            conversation_history: Vec::new(),
            last_turn_diff: None,
            current_rollout_path: None,
            last_token_usage: None,
            account_info: None,
            rate_limits: None,
            command_output_buffers: HashMap::new(),
            file_output_buffers: HashMap::new(),
            process_output_buffers: HashMap::new(),
            active_command_items: HashMap::new(),
            orchestration: OrchestrationState::default(),
            pending_local_images: Vec::new(),
            pending_remote_images: Vec::new(),
            active_personality: None,
            apps: Vec::new(),
            plugins: Vec::new(),
            skills: Vec::new(),
            models: Vec::new(),
            collaboration_modes: Vec::new(),
            active_collaboration_mode: None,
            active_supervision_notice: None,
            last_listed_thread_ids: Vec::new(),
            last_file_search_paths: Vec::new(),
            last_status_line: None,
            codex_home_override: None,
            session_overrides: SessionOverrides::default(),
            pending_selection: None,
            resume_exit_hint_emitted: false,
            raw_json,
            pending: HashMap::new(),
            next_request_id: 1,
        }
    }

    pub(crate) fn next_request_id(&mut self) -> RequestId {
        let id = self.next_request_id;
        self.next_request_id += 1;
        RequestId::Integer(id)
    }

    pub(crate) fn reset_turn_stream_state(&mut self) {
        self.command_output_buffers.clear();
        self.file_output_buffers.clear();
        self.last_agent_message = None;
        self.last_turn_diff = None;
        self.last_status_line = None;
        self.active_supervision_notice = None;
    }

    pub(crate) fn reset_thread_context(&mut self) {
        self.reset_turn_stream_state();
        self.process_output_buffers.clear();
        self.orchestration.reset_thread_context();
        self.active_turn_id = None;
        self.active_exec_process_id = None;
        self.active_async_tool_requests.clear();
        self.abandoned_async_tool_requests.clear();
        self.current_rollout_path = None;
        self.realtime_active = false;
        self.realtime_session_id = None;
        self.realtime_last_error = None;
        self.realtime_started_at = None;
        self.realtime_prompt = None;
        self.turn_running = false;
        self.activity_started_at = None;
        self.started_turn_count = 0;
        self.completed_turn_count = 0;
        self.startup_resume_picker = false;
        self.objective = None;
        self.conversation_history.clear();
        self.last_token_usage = None;
        self.active_collaboration_mode = None;
        self.pending_selection = None;
        self.active_supervision_notice = None;
    }

    pub(crate) fn replace_conversation_history(&mut self, history: Vec<ConversationMessage>) {
        self.conversation_history = history;
    }

    pub(crate) fn push_conversation_message(&mut self, role: &str, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        self.conversation_history.push(ConversationMessage {
            role: role.to_string(),
            text: trimmed.to_string(),
        });
        const MAX_CONVERSATION_MESSAGES: usize = 100;
        if self.conversation_history.len() > MAX_CONVERSATION_MESSAGES {
            let drop_count = self.conversation_history.len() - MAX_CONVERSATION_MESSAGES;
            self.conversation_history.drain(..drop_count);
        }
    }

    pub(crate) fn take_pending_attachments(&mut self) -> (Vec<String>, Vec<String>) {
        let local = std::mem::take(&mut self.pending_local_images);
        let remote = std::mem::take(&mut self.pending_remote_images);
        (local, remote)
    }

    #[cfg(test)]
    pub(crate) fn record_async_tool_request(
        &mut self,
        id: RequestId,
        tool: String,
        summary: String,
    ) {
        let worker_thread_name = fallback_async_tool_worker_name(&id);
        self.record_async_tool_request_with_timeout_and_worker(
            id,
            tool,
            summary,
            DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            worker_thread_name,
        );
    }

    #[cfg(test)]
    pub(crate) fn record_async_tool_request_with_timeout(
        &mut self,
        id: RequestId,
        tool: String,
        summary: String,
        hard_timeout: Duration,
    ) {
        let worker_thread_name = fallback_async_tool_worker_name(&id);
        self.record_async_tool_request_with_timeout_and_worker(
            id,
            tool,
            summary,
            hard_timeout,
            worker_thread_name,
        );
    }

    pub(crate) fn record_async_tool_request_with_timeout_and_worker(
        &mut self,
        id: RequestId,
        tool: String,
        summary: String,
        hard_timeout: Duration,
        worker_thread_name: String,
    ) {
        self.abandoned_async_tool_requests.remove(&id);
        self.active_async_tool_requests.insert(
            id,
            super::AsyncToolActivity {
                tool,
                summary,
                owner_kind: super::AsyncToolOwnerKind::WrapperBackgroundShell,
                source_call_id: None,
                target_background_shell_reference: None,
                target_background_shell_job_id: None,
                worker_thread_name,
                started_at: std::time::Instant::now(),
                hard_timeout,
                next_health_check_after: super::AsyncToolActivity::initial_health_check_interval(
                    hard_timeout,
                ),
            },
        );
    }

    pub(crate) fn finish_async_tool_request(
        &mut self,
        id: &RequestId,
    ) -> Option<super::AsyncToolActivity> {
        self.active_async_tool_requests.remove(id)
    }

    pub(crate) fn finish_abandoned_async_tool_request(
        &mut self,
        id: &RequestId,
    ) -> Option<AbandonedAsyncToolRequest> {
        self.abandoned_async_tool_requests.remove(id)
    }

    pub(crate) fn oldest_async_tool_activity(&self) -> Option<&super::AsyncToolActivity> {
        self.active_async_tool_requests
            .values()
            .min_by_key(|activity| activity.started_at)
    }

    pub(crate) fn oldest_async_tool_supervision_class(
        &self,
    ) -> Option<super::AsyncToolSupervisionClass> {
        self.oldest_async_tool_activity()
            .and_then(|activity| activity.supervision_class())
    }

    pub(crate) fn abandoned_async_tool_request_count(&self) -> usize {
        self.abandoned_async_tool_requests.len()
    }

    pub(crate) fn oldest_abandoned_async_tool_entry(
        &self,
    ) -> Option<(&RequestId, &AbandonedAsyncToolRequest)> {
        self.abandoned_async_tool_requests
            .iter()
            .min_by(|(left_id, left), (right_id, right)| {
                left.started_at
                    .cmp(&right.started_at)
                    .then_with(|| left.timed_out_at.cmp(&right.timed_out_at))
                    .then_with(|| left.worker_thread_name.cmp(&right.worker_thread_name))
                    .then_with(|| request_id_label(left_id).cmp(&request_id_label(right_id)))
            })
    }

    #[cfg(test)]
    pub(crate) fn oldest_abandoned_async_tool_request(
        &self,
    ) -> Option<&AbandonedAsyncToolRequest> {
        self.oldest_abandoned_async_tool_entry()
            .map(|(_, request)| request)
    }

    pub(crate) fn async_tool_worker_statuses(&self) -> Vec<AsyncToolWorkerStatus> {
        let background_shell_snapshots = self.orchestration.background_shells.snapshots();
        let mut workers = self
            .active_async_tool_requests
            .iter()
            .map(|(id, activity)| {
                let observation =
                    async_tool_observation_from_snapshots(activity, &background_shell_snapshots);
                AsyncToolWorkerStatus {
                    request_id: request_id_label(id),
                    lifecycle_state: AsyncToolWorkerLifecycleState::Running,
                    tool: activity.tool.clone(),
                    summary: activity.summary.clone(),
                    owner_kind: activity.owner_kind,
                    source_call_id: activity.source_call_id.clone(),
                    target_background_shell_reference: activity
                        .target_background_shell_reference
                        .clone(),
                    target_background_shell_job_id: activity.target_background_shell_job_id.clone(),
                    worker_thread_name: activity.worker_thread_name.clone(),
                    runtime_elapsed: activity.elapsed(),
                    state_elapsed: activity.elapsed(),
                    hard_timeout: activity.hard_timeout,
                    supervision_classification: activity.supervision_class(),
                    observation_state: Some(observation.observation_state),
                    output_state: Some(observation.output_state),
                    observed_background_shell_job: observation.observed_background_shell_job,
                    next_health_check_in: Some(activity.next_health_check_in()),
                }
            })
            .chain(
                self.abandoned_async_tool_requests
                    .iter()
                    .map(|(id, request)| {
                        let observation = async_tool_observation_from_correlation(
                            super::AsyncToolOwnerKind::WrapperBackgroundShell,
                            request.target_background_shell_job_id.as_deref(),
                            request.source_call_id.as_deref(),
                            &background_shell_snapshots,
                        );
                        AsyncToolWorkerStatus {
                            request_id: request_id_label(id),
                            lifecycle_state: AsyncToolWorkerLifecycleState::AbandonedAfterTimeout,
                            tool: request.tool.clone(),
                            summary: request.summary.clone(),
                            owner_kind: super::AsyncToolOwnerKind::WrapperBackgroundShell,
                            source_call_id: request.source_call_id.clone(),
                            target_background_shell_reference: request
                                .target_background_shell_reference
                                .clone(),
                            target_background_shell_job_id: request
                                .target_background_shell_job_id
                                .clone(),
                            worker_thread_name: request.worker_thread_name.clone(),
                            runtime_elapsed: request.elapsed_before_timeout,
                            state_elapsed: request.timed_out_elapsed(),
                            hard_timeout: request.hard_timeout,
                            supervision_classification: None,
                            observation_state: Some(observation.observation_state),
                            output_state: Some(observation.output_state),
                            observed_background_shell_job: observation
                                .observed_background_shell_job,
                            next_health_check_in: None,
                        }
                    }),
            )
            .collect::<Vec<_>>();
        workers.sort_by(|left, right| {
            left.lifecycle_state
                .cmp(&right.lifecycle_state)
                .then_with(|| right.runtime_elapsed.cmp(&left.runtime_elapsed))
                .then_with(|| left.request_id.cmp(&right.request_id))
        });
        workers
    }

    pub(crate) fn collect_due_async_tool_health_checks(
        &mut self,
    ) -> Vec<super::AsyncToolHealthCheck> {
        let now = std::time::Instant::now();
        let background_shell_snapshots = self.orchestration.background_shells.snapshots();
        let mut checks = self
            .active_async_tool_requests
            .iter_mut()
            .filter_map(|(id, activity)| {
                let elapsed = now.saturating_duration_since(activity.started_at);
                if elapsed < activity.next_health_check_after {
                    return None;
                }
                let next_interval = activity.orchestrator_health_check_interval(elapsed);
                activity.next_health_check_after = elapsed.saturating_add(next_interval);
                let observation =
                    async_tool_observation_from_snapshots(activity, &background_shell_snapshots);
                Some(super::AsyncToolHealthCheck {
                    request_id: request_id_label(id),
                    tool: activity.tool.clone(),
                    summary: activity.summary.clone(),
                    owner_kind: activity.owner_kind,
                    source_call_id: activity.source_call_id.clone(),
                    target_background_shell_reference: activity
                        .target_background_shell_reference
                        .clone(),
                    target_background_shell_job_id: activity.target_background_shell_job_id.clone(),
                    worker_thread_name: activity.worker_thread_name.clone(),
                    elapsed,
                    next_health_check_in: next_interval,
                    supervision_classification:
                        super::AsyncToolActivity::supervision_class_at_elapsed(elapsed),
                    observation_state: observation.observation_state,
                    output_state: observation.output_state,
                    observed_background_shell_job: observation.observed_background_shell_job,
                })
            })
            .collect::<Vec<_>>();
        checks.sort_by(|left, right| {
            right
                .elapsed
                .cmp(&left.elapsed)
                .then_with(|| left.request_id.cmp(&right.request_id))
        });
        checks
    }

    pub(crate) fn async_tool_backpressure_active(&self) -> bool {
        self.abandoned_async_tool_request_count() >= MAX_ABANDONED_ASYNC_TOOL_REQUESTS
    }

    pub(crate) fn expire_timed_out_async_tool_requests(&mut self) -> Vec<TimedOutAsyncToolRequest> {
        let timed_out_ids = self
            .active_async_tool_requests
            .iter()
            .filter_map(|(id, activity)| activity.timed_out().then_some(id.clone()))
            .collect::<Vec<_>>();
        let mut expired = Vec::with_capacity(timed_out_ids.len());
        for id in timed_out_ids {
            if let Some(activity) = self.active_async_tool_requests.remove(&id) {
                let elapsed = activity.elapsed();
                self.abandoned_async_tool_requests.insert(
                    id.clone(),
                    AbandonedAsyncToolRequest {
                        tool: activity.tool.clone(),
                        summary: activity.summary.clone(),
                        source_call_id: activity.source_call_id.clone(),
                        target_background_shell_reference: activity
                            .target_background_shell_reference
                            .clone(),
                        target_background_shell_job_id: activity
                            .target_background_shell_job_id
                            .clone(),
                        worker_thread_name: activity.worker_thread_name.clone(),
                        started_at: activity.started_at,
                        timed_out_at: std::time::Instant::now(),
                        elapsed_before_timeout: elapsed,
                        hard_timeout: activity.hard_timeout,
                    },
                );
                expired.push(TimedOutAsyncToolRequest {
                    id,
                    tool: activity.tool,
                    summary: activity.summary,
                    elapsed,
                    hard_timeout: activity.hard_timeout,
                });
            }
        }
        expired
    }

    pub(crate) fn current_async_tool_supervision_notice(&self) -> Option<super::SupervisionNotice> {
        let activity = self.oldest_async_tool_activity()?;
        let classification = activity.supervision_class()?;
        Some(super::SupervisionNotice {
            classification,
            tool: activity.tool.clone(),
            summary: activity.summary.clone(),
        })
    }

    pub(crate) fn refresh_async_tool_supervision_notice(
        &mut self,
    ) -> Option<super::SupervisionNoticeTransition> {
        let next_notice = self.current_async_tool_supervision_notice();
        if self.active_supervision_notice == next_notice {
            return None;
        }
        self.active_supervision_notice = next_notice.clone();
        match next_notice {
            Some(notice) => Some(super::SupervisionNoticeTransition::Raised(notice)),
            None => Some(super::SupervisionNoticeTransition::Cleared),
        }
    }
}

impl AppState {
    pub(crate) fn async_tool_observation(
        &self,
        activity: &super::AsyncToolActivity,
    ) -> AsyncToolObservation {
        let background_shell_snapshots = self.orchestration.background_shells.snapshots();
        async_tool_observation_from_snapshots(activity, &background_shell_snapshots)
    }

    pub(crate) fn abandoned_async_tool_observation(
        &self,
        request: &AbandonedAsyncToolRequest,
    ) -> AsyncToolObservation {
        let background_shell_snapshots = self.orchestration.background_shells.snapshots();
        async_tool_observation_from_correlation(
            super::AsyncToolOwnerKind::WrapperBackgroundShell,
            request.target_background_shell_job_id.as_deref(),
            request.source_call_id.as_deref(),
            &background_shell_snapshots,
        )
    }
}

fn async_tool_observation_from_snapshots(
    activity: &super::AsyncToolActivity,
    background_shell_snapshots: &[crate::background_shells::BackgroundShellJobSnapshot],
) -> AsyncToolObservation {
    async_tool_observation_from_correlation(
        activity.owner_kind,
        activity.target_background_shell_job_id.as_deref(),
        activity.source_call_id.as_deref(),
        background_shell_snapshots,
    )
}

fn async_tool_observation_from_correlation(
    owner_kind: super::AsyncToolOwnerKind,
    target_background_shell_job_id: Option<&str>,
    source_call_id: Option<&str>,
    background_shell_snapshots: &[crate::background_shells::BackgroundShellJobSnapshot],
) -> AsyncToolObservation {
    let observed_background_shell_job = observed_background_shell_job_from_snapshots(
        target_background_shell_job_id,
        source_call_id,
        background_shell_snapshots,
    )
    .map(AsyncToolObservedBackgroundShellJob::from_snapshot);
    let observation_state = match observed_background_shell_job.as_ref() {
        Some(job) if job.status != "running" => {
            AsyncToolObservationState::WrapperBackgroundShellTerminalWithoutToolResponse
        }
        Some(job) if job.total_lines > 0 => {
            AsyncToolObservationState::WrapperBackgroundShellStreamingOutput
        }
        Some(_) => AsyncToolObservationState::WrapperBackgroundShellStartedNoOutputYet,
        None => AsyncToolObservationState::NoJobOrOutputObservedYet,
    };
    let output_state = observed_background_shell_job
        .as_ref()
        .map(|job| job.output_state())
        .unwrap_or(AsyncToolOutputState::NoOutputObservedYet);
    AsyncToolObservation {
        owner_kind,
        observation_state,
        output_state,
        observed_background_shell_job,
    }
}

fn observed_background_shell_job_from_snapshots(
    target_background_shell_job_id: Option<&str>,
    source_call_id: Option<&str>,
    background_shell_snapshots: &[crate::background_shells::BackgroundShellJobSnapshot],
) -> Option<crate::background_shells::BackgroundShellJobSnapshot> {
    if let Some(job_id) = target_background_shell_job_id {
        if let Some(snapshot) = background_shell_snapshots
            .iter()
            .find(|snapshot| snapshot.id == job_id)
        {
            return Some(snapshot.clone());
        }
    }
    let source_call_id = source_call_id?;
    background_shell_snapshots
        .iter()
        .filter(|snapshot| snapshot.origin.source_call_id.as_deref() == Some(source_call_id))
        .max_by(|left, right| {
            left.total_lines
                .cmp(&right.total_lines)
                .then_with(|| left.id.cmp(&right.id))
        })
        .cloned()
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(true, false)
    }
}

pub(crate) fn request_id_label(id: &RequestId) -> String {
    match id {
        RequestId::Integer(value) => value.to_string(),
        RequestId::String(value) => value.clone(),
    }
}

#[cfg(test)]
fn fallback_async_tool_worker_name(id: &RequestId) -> String {
    format!("codexw-async-tool-worker-{}", request_id_label(id))
}
