use crate::state::AppState;

use super::LocalApiAsyncToolBackpressure;
use super::LocalApiAsyncToolSupervision;
use super::LocalApiAsyncToolWorker;
use super::LocalApiObservedBackgroundShellJob;
use super::LocalApiRecoveryOption;
use super::LocalApiRecoveryPolicy;
use super::LocalApiSupervisionNotice;

pub(super) fn async_tool_supervision_snapshot(
    session_id: &str,
    cwd: &str,
    state: &AppState,
) -> Option<LocalApiAsyncToolSupervision> {
    let (request_id, activity) = state.oldest_async_tool_entry()?;
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
        request_id: crate::state::request_id_label(request_id),
        thread_name: activity.worker_thread_name.clone(),
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

pub(super) fn async_tool_backpressure_snapshot(
    session_id: &str,
    cwd: &str,
    state: &AppState,
) -> Option<LocalApiAsyncToolBackpressure> {
    let (request_id, abandoned) = state.oldest_abandoned_async_tool_entry()?;
    let observation = state.abandoned_async_tool_observation(abandoned);
    Some(LocalApiAsyncToolBackpressure {
        abandoned_request_count: state.abandoned_async_tool_request_count(),
        saturation_threshold: crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS,
        saturated: state.async_tool_backpressure_active(),
        recommended_action: crate::supervision_recovery::async_backpressure_recommended_action(
            state,
        )
        .to_string(),
        recovery_policy: LocalApiRecoveryPolicy {
            kind: crate::supervision_recovery::async_backpressure_recovery_policy_kind(state)
                .label()
                .to_string(),
            automation_ready: crate::supervision_recovery::async_backpressure_automation_ready(
                state,
            ),
        },
        recovery_options: async_backpressure_recovery_options_snapshot(session_id, cwd, state),
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

pub(super) fn async_tool_workers_snapshot(state: &AppState) -> Vec<LocalApiAsyncToolWorker> {
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

pub(super) fn supervision_notice_snapshot(
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
        request_id: notice.request_id.clone(),
        thread_name: notice.worker_thread_name.clone(),
        owner: notice.owner_kind.label().to_string(),
        source_call_id: notice.source_call_id.clone(),
        target_background_shell_reference: notice.target_background_shell_reference.clone(),
        target_background_shell_job_id: notice.target_background_shell_job_id.clone(),
        tool: notice.tool.clone(),
        summary: notice.summary.clone(),
        observation_state: notice.observation_state.label().to_string(),
        output_state: notice.output_state.label().to_string(),
        observed_background_shell_job: notice
            .observed_background_shell_job
            .clone()
            .map(local_api_observed_background_shell_job),
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
    crate::supervision_recovery::supervision_recovery_options(
        state,
        Some(session_id),
        cwd,
        classification,
    )
    .into_iter()
    .map(|option| LocalApiRecoveryOption {
        kind: option.kind.to_string(),
        label: option.label.to_string(),
        automation_ready: option.automation_ready,
        cli_command: option.cli_command,
        local_api_method: option.local_api_method,
        local_api_path: option.local_api_path,
    })
    .collect()
}

fn async_backpressure_recovery_options_snapshot(
    session_id: &str,
    cwd: &str,
    state: &AppState,
) -> Vec<LocalApiRecoveryOption> {
    crate::supervision_recovery::async_backpressure_recovery_options(state, Some(session_id), cwd)
        .into_iter()
        .map(|option| LocalApiRecoveryOption {
            kind: option.kind.to_string(),
            label: option.label.to_string(),
            automation_ready: option.automation_ready,
            cli_command: option.cli_command,
            local_api_method: option.local_api_method,
            local_api_path: option.local_api_path,
        })
        .collect()
}
