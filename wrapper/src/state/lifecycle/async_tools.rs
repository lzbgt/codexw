use std::time::Duration;

use crate::rpc::RequestId;

use super::super::AbandonedAsyncToolRequest;
use super::super::AppState;
use super::super::AsyncToolHealthCheck;
use super::super::AsyncToolOwnerKind;
use super::super::AsyncToolSupervisionClass;
use super::super::AsyncToolWorkerLifecycleState;
use super::super::AsyncToolWorkerStatus;
#[cfg(test)]
use super::super::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT;
use super::super::MAX_ABANDONED_ASYNC_TOOL_REQUESTS;
use super::super::SupervisionNotice;
use super::super::SupervisionNoticeTransition;
use super::super::TimedOutAsyncToolRequest;
#[cfg(test)]
use super::fallback_async_tool_worker_name;
use super::request_id_label;

impl AppState {
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
            super::super::AsyncToolActivity {
                tool,
                summary,
                owner_kind: AsyncToolOwnerKind::WrapperBackgroundShell,
                source_call_id: None,
                target_background_shell_reference: None,
                target_background_shell_job_id: None,
                worker_thread_name,
                started_at: std::time::Instant::now(),
                hard_timeout,
                next_health_check_after:
                    super::super::AsyncToolActivity::initial_health_check_interval(hard_timeout),
            },
        );
    }

    pub(crate) fn finish_async_tool_request(
        &mut self,
        id: &RequestId,
    ) -> Option<super::super::AsyncToolActivity> {
        self.active_async_tool_requests.remove(id)
    }

    pub(crate) fn finish_abandoned_async_tool_request(
        &mut self,
        id: &RequestId,
    ) -> Option<AbandonedAsyncToolRequest> {
        self.abandoned_async_tool_requests.remove(id)
    }

    pub(crate) fn oldest_async_tool_activity(&self) -> Option<&super::super::AsyncToolActivity> {
        self.oldest_async_tool_entry().map(|(_, activity)| activity)
    }

    pub(crate) fn oldest_async_tool_entry(
        &self,
    ) -> Option<(&RequestId, &super::super::AsyncToolActivity)> {
        self.active_async_tool_requests
            .iter()
            .min_by(|(left_id, left), (right_id, right)| {
                left.started_at
                    .cmp(&right.started_at)
                    .then_with(|| left.worker_thread_name.cmp(&right.worker_thread_name))
                    .then_with(|| request_id_label(left_id).cmp(&request_id_label(right_id)))
            })
    }

    pub(crate) fn oldest_async_tool_supervision_class(&self) -> Option<AsyncToolSupervisionClass> {
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
    pub(crate) fn oldest_abandoned_async_tool_request(&self) -> Option<&AbandonedAsyncToolRequest> {
        self.oldest_abandoned_async_tool_entry()
            .map(|(_, request)| request)
    }

    pub(crate) fn async_tool_worker_statuses(&self) -> Vec<AsyncToolWorkerStatus> {
        let background_shell_snapshots = self.orchestration.background_shells.snapshots();
        let mut workers = self
            .active_async_tool_requests
            .iter()
            .map(|(id, activity)| {
                let observation = super::observation::async_tool_observation_from_snapshots(
                    activity,
                    &background_shell_snapshots,
                );
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
                        let observation =
                            super::observation::async_tool_observation_from_correlation(
                                AsyncToolOwnerKind::WrapperBackgroundShell,
                                request.target_background_shell_job_id.as_deref(),
                                request.source_call_id.as_deref(),
                                &background_shell_snapshots,
                            );
                        AsyncToolWorkerStatus {
                            request_id: request_id_label(id),
                            lifecycle_state: AsyncToolWorkerLifecycleState::AbandonedAfterTimeout,
                            tool: request.tool.clone(),
                            summary: request.summary.clone(),
                            owner_kind: AsyncToolOwnerKind::WrapperBackgroundShell,
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

    pub(crate) fn collect_due_async_tool_health_checks(&mut self) -> Vec<AsyncToolHealthCheck> {
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
                let observation = super::observation::async_tool_observation_from_snapshots(
                    activity,
                    &background_shell_snapshots,
                );
                Some(AsyncToolHealthCheck {
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
                        super::super::AsyncToolActivity::supervision_class_at_elapsed(elapsed),
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

    pub(crate) fn current_async_tool_supervision_notice(&self) -> Option<SupervisionNotice> {
        let (request_id, activity) = self.oldest_async_tool_entry()?;
        let classification = activity.supervision_class()?;
        let observation = self.async_tool_observation(activity);
        Some(SupervisionNotice {
            classification,
            request_id: request_id_label(request_id),
            worker_thread_name: activity.worker_thread_name.clone(),
            owner_kind: observation.owner_kind,
            source_call_id: activity.source_call_id.clone(),
            target_background_shell_reference: activity.target_background_shell_reference.clone(),
            target_background_shell_job_id: activity.target_background_shell_job_id.clone(),
            tool: activity.tool.clone(),
            summary: activity.summary.clone(),
            observation_state: observation.observation_state,
            output_state: observation.output_state,
            observed_background_shell_job: observation.observed_background_shell_job,
        })
    }

    pub(crate) fn refresh_async_tool_supervision_notice(
        &mut self,
    ) -> Option<SupervisionNoticeTransition> {
        let next_notice = self.current_async_tool_supervision_notice();
        if self.active_supervision_notice == next_notice {
            return None;
        }
        self.active_supervision_notice = next_notice.clone();
        match next_notice {
            Some(notice) => Some(SupervisionNoticeTransition::Raised(notice)),
            None => Some(SupervisionNoticeTransition::Cleared),
        }
    }
}
