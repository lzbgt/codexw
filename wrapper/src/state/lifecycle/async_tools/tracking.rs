#[cfg(test)]
use std::time::Duration;

use crate::rpc::RequestId;

use super::super::super::AbandonedAsyncToolRequest;
use super::super::super::AppState;
#[cfg(test)]
use super::super::super::AsyncToolOwnerKind;
use super::super::super::AsyncToolSupervisionClass;
#[cfg(test)]
use super::super::super::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT;
#[cfg(test)]
use super::super::fallback_async_tool_worker_name;
use super::super::request_id_label;

impl AppState {
    #[cfg(test)]
    pub(crate) fn record_async_tool_request(
        &mut self,
        id: RequestId,
        tool: String,
        summary: String,
    ) {
        let worker_thread_name = fallback_async_tool_worker_name(&id);
        self.record_async_tool_request_with_timeout_and_worker(
            id,
            tool,
            summary,
            DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            worker_thread_name,
        );
    }

    #[cfg(test)]
    pub(crate) fn record_async_tool_request_with_timeout(
        &mut self,
        id: RequestId,
        tool: String,
        summary: String,
        hard_timeout: Duration,
    ) {
        let worker_thread_name = fallback_async_tool_worker_name(&id);
        self.record_async_tool_request_with_timeout_and_worker(
            id,
            tool,
            summary,
            hard_timeout,
            worker_thread_name,
        );
    }

    #[cfg(test)]
    pub(crate) fn record_async_tool_request_with_timeout_and_worker(
        &mut self,
        id: RequestId,
        tool: String,
        summary: String,
        hard_timeout: Duration,
        worker_thread_name: String,
    ) {
        self.abandoned_async_tool_requests.remove(&id);
        self.active_async_tool_requests.insert(
            id,
            super::super::super::AsyncToolActivity {
                tool,
                summary,
                owner_kind: AsyncToolOwnerKind::WrapperBackgroundShell,
                source_call_id: None,
                target_background_shell_reference: None,
                target_background_shell_job_id: None,
                worker_thread_name,
                started_at: std::time::Instant::now(),
                hard_timeout,
                next_health_check_after:
                    super::super::super::AsyncToolActivity::initial_health_check_interval(
                        hard_timeout,
                    ),
            },
        );
    }

    pub(crate) fn finish_async_tool_request(
        &mut self,
        id: &RequestId,
    ) -> Option<super::super::super::AsyncToolActivity> {
        self.active_async_tool_requests.remove(id)
    }

    pub(crate) fn finish_abandoned_async_tool_request(
        &mut self,
        id: &RequestId,
    ) -> Option<AbandonedAsyncToolRequest> {
        self.abandoned_async_tool_requests.remove(id)
    }

    pub(crate) fn oldest_async_tool_activity(
        &self,
    ) -> Option<&super::super::super::AsyncToolActivity> {
        self.oldest_async_tool_entry().map(|(_, activity)| activity)
    }

    pub(crate) fn oldest_async_tool_entry(
        &self,
    ) -> Option<(&RequestId, &super::super::super::AsyncToolActivity)> {
        self.active_async_tool_requests
            .iter()
            .min_by(|(left_id, left), (right_id, right)| {
                left.started_at
                    .cmp(&right.started_at)
                    .then_with(|| left.worker_thread_name.cmp(&right.worker_thread_name))
                    .then_with(|| request_id_label(left_id).cmp(&request_id_label(right_id)))
            })
    }

    pub(crate) fn oldest_async_tool_supervision_class(&self) -> Option<AsyncToolSupervisionClass> {
        self.oldest_async_tool_activity()
            .and_then(|activity| activity.supervision_class())
    }

    pub(crate) fn abandoned_async_tool_request_count(&self) -> usize {
        self.abandoned_async_tool_requests.len()
    }

    pub(crate) fn oldest_abandoned_async_tool_entry(
        &self,
    ) -> Option<(&RequestId, &AbandonedAsyncToolRequest)> {
        self.abandoned_async_tool_requests
            .iter()
            .min_by(|(left_id, left), (right_id, right)| {
                left.started_at
                    .cmp(&right.started_at)
                    .then_with(|| left.timed_out_at.cmp(&right.timed_out_at))
                    .then_with(|| left.worker_thread_name.cmp(&right.worker_thread_name))
                    .then_with(|| request_id_label(left_id).cmp(&request_id_label(right_id)))
            })
    }

    #[cfg(test)]
    pub(crate) fn oldest_abandoned_async_tool_request(&self) -> Option<&AbandonedAsyncToolRequest> {
        self.oldest_abandoned_async_tool_entry()
            .map(|(_, request)| request)
    }
}
