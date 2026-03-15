use crate::background_shells::BackgroundShellJobSnapshot;

use super::super::AbandonedAsyncToolRequest;
use super::super::AppState;
use super::super::AsyncToolActivity;
use super::super::AsyncToolObservation;
use super::super::AsyncToolObservationState;
use super::super::AsyncToolObservedBackgroundShellJob;
use super::super::AsyncToolOutputState;
use super::super::AsyncToolOwnerKind;

impl AppState {
    pub(crate) fn async_tool_observation(
        &self,
        activity: &AsyncToolActivity,
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
            AsyncToolOwnerKind::WrapperBackgroundShell,
            request.target_background_shell_job_id.as_deref(),
            request.source_call_id.as_deref(),
            &background_shell_snapshots,
        )
    }
}

pub(super) fn async_tool_observation_from_snapshots(
    activity: &AsyncToolActivity,
    background_shell_snapshots: &[BackgroundShellJobSnapshot],
) -> AsyncToolObservation {
    async_tool_observation_from_correlation(
        activity.owner_kind,
        activity.target_background_shell_job_id.as_deref(),
        activity.source_call_id.as_deref(),
        background_shell_snapshots,
    )
}

pub(super) fn async_tool_observation_from_correlation(
    owner_kind: AsyncToolOwnerKind,
    target_background_shell_job_id: Option<&str>,
    source_call_id: Option<&str>,
    background_shell_snapshots: &[BackgroundShellJobSnapshot],
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
    background_shell_snapshots: &[BackgroundShellJobSnapshot],
) -> Option<BackgroundShellJobSnapshot> {
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
