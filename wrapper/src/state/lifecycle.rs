use std::collections::HashMap;
use std::time::Duration;

use crate::rpc::RequestId;

use super::AppState;
use super::ConversationMessage;
#[cfg(test)]
use super::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT;
use super::OrchestrationState;
use super::SessionOverrides;
use super::TimedOutAsyncToolRequest;

impl OrchestrationState {
    pub(crate) fn reset_thread_context(&mut self) {
        self.background_terminals.clear();
        self.cached_agent_threads.clear();
        self.live_agent_tasks.clear();
    }
}

impl AppState {
    pub(crate) fn new(auto_continue: bool, raw_json: bool) -> Self {
        Self {
            thread_id: None,
            active_turn_id: None,
            active_exec_process_id: None,
            active_async_tool_requests: HashMap::new(),
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
    }

    pub(crate) fn reset_thread_context(&mut self) {
        self.reset_turn_stream_state();
        self.process_output_buffers.clear();
        self.orchestration.reset_thread_context();
        self.active_turn_id = None;
        self.active_exec_process_id = None;
        self.active_async_tool_requests.clear();
        self.current_rollout_path = None;
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

    #[cfg(test)]
    pub(crate) fn record_async_tool_request(
        &mut self,
        id: RequestId,
        tool: String,
        summary: String,
    ) {
        self.record_async_tool_request_with_timeout(
            id,
            tool,
            summary,
            DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
        );
    }

    pub(crate) fn record_async_tool_request_with_timeout(
        &mut self,
        id: RequestId,
        tool: String,
        summary: String,
        hard_timeout: Duration,
    ) {
        self.active_async_tool_requests.insert(
            id,
            super::AsyncToolActivity {
                tool,
                summary,
                started_at: std::time::Instant::now(),
                hard_timeout,
            },
        );
    }

    pub(crate) fn finish_async_tool_request(
        &mut self,
        id: &RequestId,
    ) -> Option<super::AsyncToolActivity> {
        self.active_async_tool_requests.remove(id)
    }

    pub(crate) fn oldest_async_tool_activity(&self) -> Option<&super::AsyncToolActivity> {
        self.active_async_tool_requests
            .values()
            .min_by_key(|activity| activity.started_at)
    }

    pub(crate) fn oldest_async_tool_supervision_class(
        &self,
    ) -> Option<super::AsyncToolSupervisionClass> {
        self.oldest_async_tool_activity()
            .and_then(|activity| activity.supervision_class())
    }

    pub(crate) fn expire_timed_out_async_tool_requests(&mut self) -> Vec<TimedOutAsyncToolRequest> {
        let timed_out_ids = self
            .active_async_tool_requests
            .iter()
            .filter_map(|(id, activity)| activity.timed_out().then_some(id.clone()))
            .collect::<Vec<_>>();
        let mut expired = Vec::with_capacity(timed_out_ids.len());
        for id in timed_out_ids {
            if let Some(activity) = self.active_async_tool_requests.remove(&id) {
                let elapsed = activity.elapsed();
                expired.push(TimedOutAsyncToolRequest {
                    id,
                    tool: activity.tool,
                    summary: activity.summary,
                    elapsed,
                    hard_timeout: activity.hard_timeout,
                });
            }
        }
        expired
    }

    pub(crate) fn current_async_tool_supervision_notice(&self) -> Option<super::SupervisionNotice> {
        let activity = self.oldest_async_tool_activity()?;
        let classification = activity.supervision_class()?;
        Some(super::SupervisionNotice {
            classification,
            tool: activity.tool.clone(),
            summary: activity.summary.clone(),
        })
    }

    pub(crate) fn refresh_async_tool_supervision_notice(
        &mut self,
    ) -> Option<super::SupervisionNoticeTransition> {
        let next_notice = self.current_async_tool_supervision_notice();
        if self.active_supervision_notice == next_notice {
            return None;
        }
        self.active_supervision_notice = next_notice.clone();
        match next_notice {
            Some(notice) => Some(super::SupervisionNoticeTransition::Raised(notice)),
            None => Some(super::SupervisionNoticeTransition::Cleared),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(true, false)
    }
}
