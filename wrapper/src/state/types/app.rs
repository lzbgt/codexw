use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::time::Instant;

use crate::collaboration_preset::CollaborationModePreset;
use crate::input::AppCatalogEntry;
use crate::input::PluginCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::model_catalog::ModelCatalogEntry;
use crate::requests::PendingRequest;
use crate::rpc::RequestId;

use super::async_tools::AbandonedAsyncToolRequest;
use super::async_tools::AsyncToolActivity;
use super::async_tools::SupervisionNotice;
use super::core::ConversationMessage;
use super::core::OrchestrationState;
use super::core::PendingSelection;
use super::core::ProcessOutputBuffer;
use super::core::SessionOverrides;

pub(crate) struct AppState {
    pub(crate) thread_id: Option<String>,
    pub(crate) active_turn_id: Option<String>,
    pub(crate) active_exec_process_id: Option<String>,
    pub(crate) active_async_tool_requests: HashMap<RequestId, AsyncToolActivity>,
    pub(crate) abandoned_async_tool_requests: HashMap<RequestId, AbandonedAsyncToolRequest>,
    pub(crate) realtime_active: bool,
    pub(crate) realtime_session_id: Option<String>,
    pub(crate) realtime_last_error: Option<String>,
    pub(crate) realtime_started_at: Option<Instant>,
    pub(crate) realtime_prompt: Option<String>,
    pub(crate) pending_thread_switch: bool,
    pub(crate) turn_running: bool,
    pub(crate) activity_started_at: Option<Instant>,
    pub(crate) last_server_event_at: Option<Instant>,
    pub(crate) turn_idle_notice_emitted: bool,
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
    pub(crate) active_supervision_notice: Option<SupervisionNotice>,
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
