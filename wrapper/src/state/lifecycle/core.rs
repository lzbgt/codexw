use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

use crate::rpc::RequestId;

use super::super::AppState;
use super::super::ConversationMessage;
use super::super::OrchestrationState;
use super::super::SessionOverrides;

impl OrchestrationState {
    pub(crate) fn reset_thread_context(&mut self) {
        self.background_terminals.clear();
        self.cached_agent_threads.clear();
        self.live_agent_tasks.clear();
    }
}

impl AppState {
    pub(crate) const TURN_IDLE_WARNING_THRESHOLD: Duration = Duration::from_secs(45);

    pub(crate) fn new(auto_continue: bool, raw_json: bool) -> Self {
        Self {
            thread_id: None,
            active_turn_id: None,
            active_exec_process_id: None,
            active_async_tool_requests: HashMap::new(),
            abandoned_async_tool_requests: HashMap::new(),
            realtime_active: false,
            realtime_session_id: None,
            realtime_last_error: None,
            realtime_started_at: None,
            realtime_prompt: None,
            pending_thread_switch: false,
            turn_running: false,
            activity_started_at: None,
            last_server_event_at: None,
            turn_idle_notice_emitted: false,
            started_turn_count: 0,
            completed_turn_count: 0,
            auto_continue,
            startup_resume_picker: false,
            objective: None,
            last_agent_message: None,
            conversation_history: Vec::new(),
            last_turn_diff: None,
            current_rollout_path: None,
            last_token_usage: None,
            account_info: None,
            rate_limits: None,
            command_output_buffers: HashMap::new(),
            file_output_buffers: HashMap::new(),
            process_output_buffers: HashMap::new(),
            active_command_items: HashMap::new(),
            orchestration: OrchestrationState::default(),
            pending_local_images: Vec::new(),
            pending_remote_images: Vec::new(),
            active_personality: None,
            apps: Vec::new(),
            plugins: Vec::new(),
            skills: Vec::new(),
            models: Vec::new(),
            collaboration_modes: Vec::new(),
            active_collaboration_mode: None,
            active_supervision_notice: None,
            last_listed_thread_ids: Vec::new(),
            last_file_search_paths: Vec::new(),
            last_status_line: None,
            codex_home_override: None,
            session_overrides: SessionOverrides::default(),
            pending_selection: None,
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
        self.active_supervision_notice = None;
        self.turn_idle_notice_emitted = false;
    }

    pub(crate) fn reset_thread_context(&mut self) {
        self.reset_turn_stream_state();
        self.process_output_buffers.clear();
        self.orchestration.reset_thread_context();
        self.active_turn_id = None;
        self.active_exec_process_id = None;
        self.active_async_tool_requests.clear();
        self.abandoned_async_tool_requests.clear();
        self.current_rollout_path = None;
        self.realtime_active = false;
        self.realtime_session_id = None;
        self.realtime_last_error = None;
        self.realtime_started_at = None;
        self.realtime_prompt = None;
        self.turn_running = false;
        self.activity_started_at = None;
        self.last_server_event_at = None;
        self.turn_idle_notice_emitted = false;
        self.started_turn_count = 0;
        self.completed_turn_count = 0;
        self.startup_resume_picker = false;
        self.objective = None;
        self.conversation_history.clear();
        self.last_token_usage = None;
        self.active_collaboration_mode = None;
        self.pending_selection = None;
        self.active_supervision_notice = None;
    }

    pub(crate) fn replace_conversation_history(&mut self, history: Vec<ConversationMessage>) {
        self.conversation_history = history;
    }

    pub(crate) fn push_conversation_message(&mut self, role: &str, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        self.conversation_history.push(ConversationMessage {
            role: role.to_string(),
            text: trimmed.to_string(),
        });
        const MAX_CONVERSATION_MESSAGES: usize = 100;
        if self.conversation_history.len() > MAX_CONVERSATION_MESSAGES {
            let drop_count = self.conversation_history.len() - MAX_CONVERSATION_MESSAGES;
            self.conversation_history.drain(..drop_count);
        }
    }

    pub(crate) fn take_pending_attachments(&mut self) -> (Vec<String>, Vec<String>) {
        let local = std::mem::take(&mut self.pending_local_images);
        let remote = std::mem::take(&mut self.pending_remote_images);
        (local, remote)
    }

    pub(crate) fn note_server_activity(&mut self) {
        self.last_server_event_at = Some(Instant::now());
        self.turn_idle_notice_emitted = false;
    }

    pub(crate) fn stalled_turn_idle_for(&self) -> Option<Duration> {
        if !self.turn_running {
            return None;
        }
        let idle = Instant::now().saturating_duration_since(self.last_server_event_at?);
        (idle >= Self::TURN_IDLE_WARNING_THRESHOLD).then_some(idle)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(true, false)
    }
}
