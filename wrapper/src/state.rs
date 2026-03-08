use std::collections::HashMap;
use std::time::Instant;

use crate::collaboration_preset::CollaborationModePreset;
use crate::input::AppCatalogEntry;
use crate::input::PluginCatalogEntry;
use crate::input::SkillCatalogEntry;
use crate::model_catalog::ModelCatalogEntry;
use crate::requests::PendingRequest;
use crate::rpc::RequestId;
pub(crate) use crate::state_helpers::buffer_item_delta;
pub(crate) use crate::state_helpers::buffer_process_delta;
pub(crate) use crate::state_helpers::canonicalize_or_keep;
pub(crate) use crate::state_helpers::emit_status_line;
pub(crate) use crate::state_helpers::get_string;
pub(crate) use crate::state_helpers::summarize_text;
pub(crate) use crate::state_helpers::thread_id;

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
    pub(crate) startup_resume_picker: bool,
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
    pub(crate) resume_exit_hint_emitted: bool,
    pub(crate) raw_json: bool,
    pub(crate) pending: HashMap<RequestId, PendingRequest>,
    pub(crate) next_request_id: i64,
}

impl AppState {
    pub(crate) fn new(auto_continue: bool, raw_json: bool) -> Self {
        Self {
            thread_id: None,
            active_turn_id: None,
            active_exec_process_id: None,
            realtime_active: false,
            realtime_session_id: None,
            realtime_last_error: None,
            realtime_started_at: None,
            realtime_prompt: None,
            pending_thread_switch: false,
            turn_running: false,
            activity_started_at: None,
            started_turn_count: 0,
            completed_turn_count: 0,
            auto_continue,
            startup_resume_picker: false,
            objective: None,
            last_agent_message: None,
            last_turn_diff: None,
            last_token_usage: None,
            account_info: None,
            rate_limits: None,
            command_output_buffers: HashMap::new(),
            file_output_buffers: HashMap::new(),
            process_output_buffers: HashMap::new(),
            pending_local_images: Vec::new(),
            pending_remote_images: Vec::new(),
            active_personality: None,
            apps: Vec::new(),
            plugins: Vec::new(),
            skills: Vec::new(),
            models: Vec::new(),
            collaboration_modes: Vec::new(),
            active_collaboration_mode: None,
            last_listed_thread_ids: Vec::new(),
            last_file_search_paths: Vec::new(),
            last_status_line: None,
            resume_exit_hint_emitted: false,
            raw_json,
            pending: HashMap::new(),
            next_request_id: 1,
        }
    }

    pub(crate) fn next_request_id(&mut self) -> RequestId {
        let id = self.next_request_id;
        self.next_request_id += 1;
        RequestId::Integer(id)
    }

    pub(crate) fn reset_turn_stream_state(&mut self) {
        self.command_output_buffers.clear();
        self.file_output_buffers.clear();
        self.last_agent_message = None;
        self.last_turn_diff = None;
        self.last_status_line = None;
    }

    pub(crate) fn reset_thread_context(&mut self) {
        self.reset_turn_stream_state();
        self.process_output_buffers.clear();
        self.active_turn_id = None;
        self.active_exec_process_id = None;
        self.realtime_active = false;
        self.realtime_session_id = None;
        self.realtime_last_error = None;
        self.realtime_started_at = None;
        self.realtime_prompt = None;
        self.turn_running = false;
        self.activity_started_at = None;
        self.started_turn_count = 0;
        self.completed_turn_count = 0;
        self.startup_resume_picker = false;
        self.objective = None;
        self.last_token_usage = None;
        self.active_personality = None;
        self.active_collaboration_mode = None;
    }

    pub(crate) fn take_pending_attachments(&mut self) -> (Vec<String>, Vec<String>) {
        let local = std::mem::take(&mut self.pending_local_images);
        let remote = std::mem::take(&mut self.pending_remote_images);
        (local, remote)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(true, false)
    }
}
