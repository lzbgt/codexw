use std::collections::HashMap;
use std::time::Instant;

use crate::collaboration_preset::CollaborationModePreset;
use crate::input::AppCatalogEntry;
use crate::input::PluginCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::model_catalog::ModelCatalogEntry;
use crate::requests::PendingRequest;
use crate::rpc::RequestId;

#[derive(Default)]
pub(crate) struct ProcessOutputBuffer {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) struct AppState {
    pub(crate) thread_id: Option<String>,
    pub(crate) active_turn_id: Option<String>,
    pub(crate) active_exec_process_id: Option<String>,
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
    pub(crate) objective: Option<String>,
    pub(crate) last_agent_message: Option<String>,
    pub(crate) last_turn_diff: Option<String>,
    pub(crate) last_token_usage: Option<serde_json::Value>,
    pub(crate) account_info: Option<serde_json::Value>,
    pub(crate) rate_limits: Option<serde_json::Value>,
    pub(crate) command_output_buffers: HashMap<String, String>,
    pub(crate) file_output_buffers: HashMap<String, String>,
    pub(crate) process_output_buffers: HashMap<String, ProcessOutputBuffer>,
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
    pub(crate) raw_json: bool,
    pub(crate) pending: HashMap<RequestId, PendingRequest>,
    pub(crate) next_request_id: i64,
}
