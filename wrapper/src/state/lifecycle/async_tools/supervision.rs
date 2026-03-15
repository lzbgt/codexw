use crate::state::AbandonedAsyncToolRequest;
use crate::state::AppState;
use crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS;
use crate::state::SupervisionNotice;
use crate::state::SupervisionNoticeTransition;
use crate::state::TimedOutAsyncToolRequest;

use super::super::request_id_label;

impl AppState {
    pub(crate) fn async_tool_backpressure_active(&self) -> bool {
        self.abandoned_async_tool_request_count() >= MAX_ABANDONED_ASYNC_TOOL_REQUESTS
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
                self.abandoned_async_tool_requests.insert(
                    id.clone(),
                    AbandonedAsyncToolRequest {
                        tool: activity.tool.clone(),
                        summary: activity.summary.clone(),
                        source_call_id: activity.source_call_id.clone(),
                        target_background_shell_reference: activity
                            .target_background_shell_reference
                            .clone(),
                        target_background_shell_job_id: activity
                            .target_background_shell_job_id
                            .clone(),
                        worker_thread_name: activity.worker_thread_name.clone(),
                        started_at: activity.started_at,
                        timed_out_at: std::time::Instant::now(),
                        elapsed_before_timeout: elapsed,
                        hard_timeout: activity.hard_timeout,
                    },
                );
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

    pub(crate) fn current_async_tool_supervision_notice(&self) -> Option<SupervisionNotice> {
        let (request_id, activity) = self.oldest_async_tool_entry()?;
        let classification = activity.supervision_class()?;
        let observation = self.async_tool_observation(activity);
        Some(SupervisionNotice {
            classification,
            request_id: request_id_label(request_id),
            worker_thread_name: activity.worker_thread_name.clone(),
            owner_kind: observation.owner_kind,
            source_call_id: activity.source_call_id.clone(),
            target_background_shell_reference: activity.target_background_shell_reference.clone(),
            target_background_shell_job_id: activity.target_background_shell_job_id.clone(),
            tool: activity.tool.clone(),
            summary: activity.summary.clone(),
            observation_state: observation.observation_state,
            output_state: observation.output_state,
            observed_background_shell_job: observation.observed_background_shell_job,
        })
    }

    pub(crate) fn refresh_async_tool_supervision_notice(
        &mut self,
    ) -> Option<SupervisionNoticeTransition> {
        let next_notice = self.current_async_tool_supervision_notice();
        if self.active_supervision_notice == next_notice {
            return None;
        }
        self.active_supervision_notice = next_notice.clone();
        match next_notice {
            Some(notice) => Some(SupervisionNoticeTransition::Raised(notice)),
            None => Some(SupervisionNoticeTransition::Cleared),
        }
    }
}
