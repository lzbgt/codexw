#[path = "background_shells/core.rs"]
mod core;
#[path = "background_shells/execution.rs"]
mod execution;
#[path = "background_shells/recipes.rs"]
mod recipes;
#[path = "background_shells/services.rs"]
mod services;
#[path = "background_shells/snapshots.rs"]
mod snapshots;

#[cfg(test)]
#[path = "background_shells/tests.rs"]
mod tests;

pub(crate) use self::core::BackgroundShellCapabilityDependencyState;
pub(crate) use self::core::BackgroundShellCapabilityDependencySummary;
pub(crate) use self::core::BackgroundShellCapabilityIssueClass;
pub(crate) use self::core::BackgroundShellIntent;
pub(crate) use self::core::BackgroundShellInteractionAction;
pub(crate) use self::core::BackgroundShellInteractionParameter;
pub(crate) use self::core::BackgroundShellInteractionRecipe;
pub(crate) use self::core::BackgroundShellJobSnapshot;
pub(crate) use self::core::BackgroundShellJobState;
pub(crate) use self::core::BackgroundShellJobStatus;
pub(crate) use self::core::BackgroundShellManager;
pub(crate) use self::core::BackgroundShellOrigin;
pub(crate) use self::core::BackgroundShellOutputLine;
pub(crate) use self::core::BackgroundShellReadyWaitOutcome;
pub(crate) use self::core::BackgroundShellServiceIssueClass;
pub(crate) use self::core::BackgroundShellServiceReadiness;
pub(crate) use self::core::DEFAULT_POLL_LIMIT;
pub(crate) use self::core::DEFAULT_READY_WAIT_TIMEOUT_MS;
pub(crate) use self::core::MAX_POLL_LIMIT;
pub(crate) use self::core::MAX_STORED_LINES;
pub(crate) use self::core::READY_WAIT_POLL_INTERVAL_MS;
pub(crate) use self::core::terminate_pid;
pub(crate) use self::execution::parse_background_shell_optional_string;
pub(crate) use self::execution::terminate_jobs;
pub(crate) use self::execution::validate_service_capability;
pub(crate) use self::recipes::parse_background_shell_interaction_recipes;
pub(crate) use self::snapshots::exit_code;
pub(crate) use self::snapshots::service_readiness_for_state;
pub(crate) use self::snapshots::status_label;
