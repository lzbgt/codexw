use std::time::Duration;
use std::time::Instant;

use crate::background_shells::BackgroundShellJobSnapshot;
use crate::rpc::RequestId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AsyncToolSupervisionClass {
    ToolSlow,
    ToolWedged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AsyncToolOwnerKind {
    WrapperBackgroundShell,
}

impl AsyncToolOwnerKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::WrapperBackgroundShell => "wrapper_background_shell",
        }
    }

    pub(crate) fn prompt_label(self) -> &'static str {
        match self {
            Self::WrapperBackgroundShell => "wrapper bg shell",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AsyncToolWorkerLifecycleState {
    Running,
    AbandonedAfterTimeout,
}

impl AsyncToolWorkerLifecycleState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::AbandonedAfterTimeout => "abandoned_after_timeout",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SupervisionRecoveryPolicyKind {
    WarnOnly,
    OperatorInterruptOrExitResume,
}

impl SupervisionRecoveryPolicyKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::WarnOnly => "warn_only",
            Self::OperatorInterruptOrExitResume => "operator_interrupt_or_exit_resume",
        }
    }
}

impl AsyncToolSupervisionClass {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::ToolSlow => "tool_slow",
            Self::ToolWedged => "tool_wedged",
        }
    }

    pub(crate) fn recommended_action(self) -> &'static str {
        match self {
            Self::ToolSlow => "observe_or_interrupt",
            Self::ToolWedged => "interrupt_or_exit_resume",
        }
    }

    pub(crate) fn prompt_hint(self) -> &'static str {
        match self {
            Self::ToolSlow => "observe or interrupt",
            Self::ToolWedged => "interrupt or exit",
        }
    }

    pub(crate) fn recovery_policy_kind(self) -> SupervisionRecoveryPolicyKind {
        match self {
            Self::ToolSlow => SupervisionRecoveryPolicyKind::WarnOnly,
            Self::ToolWedged => SupervisionRecoveryPolicyKind::OperatorInterruptOrExitResume,
        }
    }

    pub(crate) fn automation_ready(self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SupervisionNotice {
    pub(crate) classification: AsyncToolSupervisionClass,
    pub(crate) request_id: String,
    pub(crate) worker_thread_name: String,
    pub(crate) owner_kind: AsyncToolOwnerKind,
    pub(crate) source_call_id: Option<String>,
    pub(crate) target_background_shell_reference: Option<String>,
    pub(crate) target_background_shell_job_id: Option<String>,
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) observation_state: AsyncToolObservationState,
    pub(crate) output_state: AsyncToolOutputState,
    pub(crate) observed_background_shell_job: Option<AsyncToolObservedBackgroundShellJob>,
}

impl SupervisionNotice {
    pub(crate) fn recommended_action(&self) -> &'static str {
        self.classification.recommended_action()
    }

    pub(crate) fn recovery_policy_kind(&self) -> SupervisionRecoveryPolicyKind {
        self.classification.recovery_policy_kind()
    }

    pub(crate) fn automation_ready(&self) -> bool {
        self.classification.automation_ready()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SupervisionNoticeTransition {
    Raised(SupervisionNotice),
    Cleared,
}

pub(crate) const ASYNC_TOOL_SLOW_THRESHOLD: Duration = Duration::from_secs(15);
pub(crate) const ASYNC_TOOL_WEDGED_THRESHOLD: Duration = Duration::from_secs(60);
pub(crate) const ASYNC_TOOL_OUTPUT_STALE_THRESHOLD: Duration = Duration::from_secs(30);
pub(crate) const MAX_ABANDONED_ASYNC_TOOL_REQUESTS: usize = 2;
#[cfg(test)]
pub(crate) const DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub(crate) struct AsyncToolActivity {
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) owner_kind: AsyncToolOwnerKind,
    pub(crate) source_call_id: Option<String>,
    pub(crate) target_background_shell_reference: Option<String>,
    pub(crate) target_background_shell_job_id: Option<String>,
    pub(crate) worker_thread_name: String,
    pub(crate) started_at: Instant,
    pub(crate) hard_timeout: Duration,
    pub(crate) next_health_check_after: Duration,
}

impl AsyncToolActivity {
    pub(crate) fn initial_health_check_interval(hard_timeout: Duration) -> Duration {
        (hard_timeout / 3)
            .max(Duration::from_secs(3))
            .min(Duration::from_secs(15))
    }

    pub(crate) fn orchestrator_health_check_interval(&self, elapsed: Duration) -> Duration {
        if elapsed >= ASYNC_TOOL_WEDGED_THRESHOLD {
            Duration::from_secs(30)
        } else if elapsed >= ASYNC_TOOL_SLOW_THRESHOLD {
            Duration::from_secs(15)
        } else {
            Self::initial_health_check_interval(self.hard_timeout)
        }
    }

    pub(crate) fn elapsed(&self) -> Duration {
        Instant::now().saturating_duration_since(self.started_at)
    }

    pub(crate) fn supervision_class_at_elapsed(
        elapsed: Duration,
    ) -> Option<AsyncToolSupervisionClass> {
        if elapsed >= ASYNC_TOOL_WEDGED_THRESHOLD {
            Some(AsyncToolSupervisionClass::ToolWedged)
        } else if elapsed >= ASYNC_TOOL_SLOW_THRESHOLD {
            Some(AsyncToolSupervisionClass::ToolSlow)
        } else {
            None
        }
    }

    pub(crate) fn timed_out(&self) -> bool {
        self.elapsed() >= self.hard_timeout
    }

    pub(crate) fn supervision_class(&self) -> Option<AsyncToolSupervisionClass> {
        Self::supervision_class_at_elapsed(self.elapsed())
    }

    pub(crate) fn next_health_check_in(&self) -> Duration {
        self.next_health_check_after.saturating_sub(self.elapsed())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AsyncToolHealthCheck {
    pub(crate) request_id: String,
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) owner_kind: AsyncToolOwnerKind,
    pub(crate) source_call_id: Option<String>,
    pub(crate) target_background_shell_reference: Option<String>,
    pub(crate) target_background_shell_job_id: Option<String>,
    pub(crate) worker_thread_name: String,
    pub(crate) elapsed: Duration,
    pub(crate) next_health_check_in: Duration,
    pub(crate) supervision_classification: Option<AsyncToolSupervisionClass>,
    pub(crate) observation_state: AsyncToolObservationState,
    pub(crate) output_state: AsyncToolOutputState,
    pub(crate) observed_background_shell_job: Option<AsyncToolObservedBackgroundShellJob>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AsyncToolObservationState {
    NoJobOrOutputObservedYet,
    WrapperBackgroundShellStartedNoOutputYet,
    WrapperBackgroundShellStreamingOutput,
    WrapperBackgroundShellTerminalWithoutToolResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AsyncToolOutputState {
    NoOutputObservedYet,
    RecentOutputObserved,
    StaleOutputObserved,
}

impl AsyncToolOutputState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::NoOutputObservedYet => "no_output_observed_yet",
            Self::RecentOutputObserved => "recent_output_observed",
            Self::StaleOutputObserved => "stale_output_observed",
        }
    }

    pub(crate) fn prompt_label(self) -> &'static str {
        match self {
            Self::NoOutputObservedYet => "no output yet",
            Self::RecentOutputObserved => "recent output",
            Self::StaleOutputObserved => "stale output",
        }
    }
}

impl AsyncToolObservationState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::NoJobOrOutputObservedYet => "no_job_or_output_observed_yet",
            Self::WrapperBackgroundShellStartedNoOutputYet => {
                "wrapper_background_shell_started_no_output_yet"
            }
            Self::WrapperBackgroundShellStreamingOutput => {
                "wrapper_background_shell_streaming_output"
            }
            Self::WrapperBackgroundShellTerminalWithoutToolResponse => {
                "wrapper_background_shell_terminal_without_tool_response"
            }
        }
    }

    pub(crate) fn prompt_label(self) -> &'static str {
        match self {
            Self::NoJobOrOutputObservedYet => "awaiting shell start/output",
            Self::WrapperBackgroundShellStartedNoOutputYet => "job started; awaiting output",
            Self::WrapperBackgroundShellStreamingOutput => "job streaming output",
            Self::WrapperBackgroundShellTerminalWithoutToolResponse => {
                "job ended; awaiting tool response"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AsyncToolObservedBackgroundShellJob {
    pub(crate) job_id: String,
    pub(crate) status: String,
    pub(crate) command: String,
    pub(crate) total_lines: u64,
    pub(crate) last_output_age: Option<Duration>,
    pub(crate) recent_lines: Vec<String>,
}

impl AsyncToolObservedBackgroundShellJob {
    pub(crate) fn from_snapshot(snapshot: BackgroundShellJobSnapshot) -> Self {
        Self {
            job_id: snapshot.id,
            status: snapshot.status,
            command: snapshot.command,
            total_lines: snapshot.total_lines,
            last_output_age: snapshot.last_output_age,
            recent_lines: snapshot.recent_lines,
        }
    }

    pub(crate) fn latest_output_preview(&self) -> Option<&str> {
        self.recent_lines
            .iter()
            .rev()
            .find(|line| !line.trim().is_empty())
            .map(String::as_str)
    }

    pub(crate) fn output_state(&self) -> AsyncToolOutputState {
        if self.total_lines == 0 {
            return AsyncToolOutputState::NoOutputObservedYet;
        }
        match self.last_output_age {
            Some(age) if age <= ASYNC_TOOL_OUTPUT_STALE_THRESHOLD => {
                AsyncToolOutputState::RecentOutputObserved
            }
            Some(_) | None => AsyncToolOutputState::StaleOutputObserved,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AsyncToolObservation {
    pub(crate) owner_kind: AsyncToolOwnerKind,
    pub(crate) observation_state: AsyncToolObservationState,
    pub(crate) output_state: AsyncToolOutputState,
    pub(crate) observed_background_shell_job: Option<AsyncToolObservedBackgroundShellJob>,
}

#[derive(Debug, Clone)]
pub(crate) struct TimedOutAsyncToolRequest {
    pub(crate) id: RequestId,
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) elapsed: Duration,
    pub(crate) hard_timeout: Duration,
}

#[derive(Debug, Clone)]
pub(crate) struct AbandonedAsyncToolRequest {
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) source_call_id: Option<String>,
    pub(crate) target_background_shell_reference: Option<String>,
    pub(crate) target_background_shell_job_id: Option<String>,
    pub(crate) worker_thread_name: String,
    pub(crate) started_at: Instant,
    pub(crate) timed_out_at: Instant,
    pub(crate) elapsed_before_timeout: Duration,
    pub(crate) hard_timeout: Duration,
}

impl AbandonedAsyncToolRequest {
    pub(crate) fn timed_out_elapsed(&self) -> Duration {
        Instant::now().saturating_duration_since(self.timed_out_at)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AsyncToolWorkerStatus {
    pub(crate) request_id: String,
    pub(crate) lifecycle_state: AsyncToolWorkerLifecycleState,
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) owner_kind: AsyncToolOwnerKind,
    pub(crate) source_call_id: Option<String>,
    pub(crate) target_background_shell_reference: Option<String>,
    pub(crate) target_background_shell_job_id: Option<String>,
    pub(crate) worker_thread_name: String,
    pub(crate) runtime_elapsed: Duration,
    pub(crate) state_elapsed: Duration,
    pub(crate) hard_timeout: Duration,
    pub(crate) supervision_classification: Option<AsyncToolSupervisionClass>,
    pub(crate) observation_state: Option<AsyncToolObservationState>,
    pub(crate) output_state: Option<AsyncToolOutputState>,
    pub(crate) observed_background_shell_job: Option<AsyncToolObservedBackgroundShellJob>,
    pub(crate) next_health_check_in: Option<Duration>,
}
