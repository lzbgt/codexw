use crate::state::AppState;
use crate::state::AsyncToolHealthCheck;
use crate::state::AsyncToolOwnerKind;
use crate::state::AsyncToolWorkerLifecycleState;
use crate::state::AsyncToolWorkerStatus;

use super::super::request_id_label;

impl AppState {
    pub(crate) fn async_tool_worker_statuses(&self) -> Vec<AsyncToolWorkerStatus> {
        let background_shell_snapshots = self.orchestration.background_shells.snapshots();
        let mut workers = self
            .active_async_tool_requests
            .iter()
            .map(|(id, activity)| {
                let observation = super::super::observation::async_tool_observation_from_snapshots(
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
                            super::super::observation::async_tool_observation_from_correlation(
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
                let observation = super::super::observation::async_tool_observation_from_snapshots(
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
                        crate::state::AsyncToolActivity::supervision_class_at_elapsed(elapsed),
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
}
