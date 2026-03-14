use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use crate::background_shells::BackgroundShellManager;
use crate::background_terminals::BackgroundTerminalSummary;
use crate::collaboration_preset::CollaborationModePreset;
use crate::input::AppCatalogEntry;
use crate::input::PluginCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::model_catalog::ModelCatalogEntry;
use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::orchestration_view::CachedAgentThreadSummary;
use crate::requests::PendingRequest;
use crate::rpc::RequestId;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AsyncToolSupervisionClass {
    ToolSlow,
    ToolWedged,
}

impl AsyncToolSupervisionClass {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::ToolSlow => "tool_slow",
            Self::ToolWedged => "tool_wedged",
        }
    }
}

pub(crate) const ASYNC_TOOL_SLOW_THRESHOLD: Duration = Duration::from_secs(15);
pub(crate) const ASYNC_TOOL_WEDGED_THRESHOLD: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub(crate) struct AsyncToolActivity {
    pub(crate) tool: String,
    pub(crate) summary: String,
    pub(crate) started_at: Instant,
}

impl AsyncToolActivity {
    pub(crate) fn elapsed(&self) -> Duration {
        Instant::now().saturating_duration_since(self.started_at)
    }

    pub(crate) fn supervision_class(&self) -> Option<AsyncToolSupervisionClass> {
        let elapsed = self.elapsed();
        if elapsed >= ASYNC_TOOL_WEDGED_THRESHOLD {
            Some(AsyncToolSupervisionClass::ToolWedged)
        } else if elapsed >= ASYNC_TOOL_SLOW_THRESHOLD {
            Some(AsyncToolSupervisionClass::ToolSlow)
        } else {
            None
        }
    }
}

pub(crate) struct AppState {
    pub(crate) thread_id: Option<String>,
    pub(crate) active_turn_id: Option<String>,
    pub(crate) active_exec_process_id: Option<String>,
    pub(crate) active_async_tool_requests: HashMap<RequestId, AsyncToolActivity>,
    pub(crate) realtime_active: bool,
    pub(crate) realtime_session_id: Option<String>,
    pub(crate) realtime_last_error: Option<String>,
    pub(crate) realtime_started_at: Option<Instant>,
    pub(crate) realtime_prompt: Option<String>,
    pub(crate) pending_thread_switch: bool,
    pub(crate) turn_running: bool,
    pub(crate) activity_started_at: Option<Instant>,
    pub(crate) started_turn_count: u64,
    pub(crate) completed_turn_count: u64,
    pub(crate) auto_continue: bool,
    pub(crate) startup_resume_picker: bool,
    pub(crate) objective: Option<String>,
    pub(crate) last_agent_message: Option<String>,
    pub(crate) conversation_history: Vec<ConversationMessage>,
    pub(crate) last_turn_diff: Option<String>,
    pub(crate) current_rollout_path: Option<PathBuf>,
    pub(crate) last_token_usage: Option<serde_json::Value>,
    pub(crate) account_info: Option<serde_json::Value>,
    pub(crate) rate_limits: Option<serde_json::Value>,
    pub(crate) command_output_buffers: HashMap<String, String>,
    pub(crate) file_output_buffers: HashMap<String, String>,
    pub(crate) process_output_buffers: HashMap<String, ProcessOutputBuffer>,
    pub(crate) active_command_items: HashMap<String, String>,
    pub(crate) orchestration: OrchestrationState,
    pub(crate) pending_local_images: Vec<String>,
    pub(crate) pending_remote_images: Vec<String>,
    pub(crate) active_personality: Option<String>,
    pub(crate) apps: Vec<AppCatalogEntry>,
    pub(crate) plugins: Vec<PluginCatalogEntry>,
    pub(crate) skills: Vec<SkillCatalogEntry>,
    pub(crate) models: Vec<ModelCatalogEntry>,
    pub(crate) collaboration_modes: Vec<CollaborationModePreset>,
    pub(crate) active_collaboration_mode: Option<CollaborationModePreset>,
    pub(crate) last_listed_thread_ids: Vec<String>,
    pub(crate) last_file_search_paths: Vec<String>,
    pub(crate) last_status_line: Option<String>,
    pub(crate) codex_home_override: Option<PathBuf>,
    pub(crate) session_overrides: SessionOverrides,
    pub(crate) pending_selection: Option<PendingSelection>,
    pub(crate) resume_exit_hint_emitted: bool,
    pub(crate) raw_json: bool,
    pub(crate) pending: HashMap<RequestId, PendingRequest>,
    pub(crate) next_request_id: i64,
}

impl Deref for AppState {
    type Target = OrchestrationState;

    fn deref(&self) -> &Self::Target {
        &self.orchestration
    }
}

impl DerefMut for AppState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.orchestration
    }
}
