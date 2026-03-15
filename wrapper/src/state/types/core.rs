use std::collections::HashMap;

use crate::background_shells::BackgroundShellManager;
use crate::background_terminals::BackgroundTerminalSummary;
use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::orchestration_view::CachedAgentThreadSummary;

#[derive(Default)]
pub(crate) struct ProcessOutputBuffer {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SessionOverrides {
    pub(crate) model: Option<Option<String>>,
    pub(crate) reasoning_effort: Option<Option<String>>,
    pub(crate) service_tier: Option<Option<String>>,
    pub(crate) personality: Option<Option<String>>,
    pub(crate) approval_policy: Option<String>,
    pub(crate) thread_sandbox_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingSelection {
    Model,
    ReasoningEffort { model_id: String },
    Personality,
    Permissions,
    Theme,
}

#[derive(Default)]
pub(crate) struct OrchestrationState {
    pub(crate) background_terminals: HashMap<String, BackgroundTerminalSummary>,
    pub(crate) background_shells: BackgroundShellManager,
    pub(crate) cached_agent_threads: Vec<CachedAgentThreadSummary>,
    pub(crate) live_agent_tasks: HashMap<String, LiveAgentTaskSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConversationMessage {
    pub(crate) role: String,
    pub(crate) text: String,
}
