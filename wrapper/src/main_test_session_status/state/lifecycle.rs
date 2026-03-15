use super::*;

#[test]
fn resetting_thread_context_clears_stream_buffers() {
    let mut state = crate::state::AppState::new(true, false);
    state
        .command_output_buffers
        .insert("cmd-1".to_string(), "out".to_string());
    state
        .file_output_buffers
        .insert("file-1".to_string(), "diff".to_string());
    state.process_output_buffers.insert(
        "proc-1".to_string(),
        crate::state::ProcessOutputBuffer {
            stdout: "stdout".to_string(),
            stderr: "stderr".to_string(),
        },
    );
    state.last_agent_message = Some("reply".to_string());
    state.last_turn_diff = Some("diff".to_string());
    state.last_status_line = Some("running".to_string());
    state.record_async_tool_request(
        crate::rpc::RequestId::Integer(4),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
    );
    state.live_agent_tasks.insert(
        "call-1".to_string(),
        LiveAgentTaskSummary {
            id: "call-1".to_string(),
            tool: "spawnAgent".to_string(),
            status: "inProgress".to_string(),
            sender_thread_id: "thread-main".to_string(),
            receiver_thread_ids: vec!["thread-agent-1".to_string()],
            prompt: Some("inspect auth".to_string()),
            agent_statuses: std::collections::BTreeMap::from([(
                "thread-agent-1".to_string(),
                "running".to_string(),
            )]),
        },
    );

    state.reset_thread_context();

    assert!(state.command_output_buffers.is_empty());
    assert!(state.file_output_buffers.is_empty());
    assert!(state.process_output_buffers.is_empty());
    assert!(state.last_agent_message.is_none());
    assert!(state.last_turn_diff.is_none());
    assert!(state.last_status_line.is_none());
    assert!(state.active_async_tool_requests.is_empty());
    assert!(state.live_agent_tasks.is_empty());
    assert!(!state.startup_resume_picker);
}

#[test]
fn finishing_async_tool_request_removes_it_from_status_tracking() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request(
        crate::rpc::RequestId::Integer(9),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
    );

    let removed = state.finish_async_tool_request(&crate::rpc::RequestId::Integer(9));

    assert!(removed.is_some());
    assert!(state.active_async_tool_requests.is_empty());
    assert!(state.oldest_async_tool_activity().is_none());
}

#[test]
fn expiring_async_tool_requests_moves_them_into_abandoned_backlog() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request_with_timeout(
        crate::rpc::RequestId::Integer(15),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        std::time::Duration::from_secs(1),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(15))
    {
        activity.source_call_id = Some("call-15".to_string());
        activity.target_background_shell_reference = Some("dev.api".to_string());
        activity.target_background_shell_job_id = Some("bg-1".to_string());
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(80);
    }

    let expired = state.expire_timed_out_async_tool_requests();

    assert_eq!(expired.len(), 1);
    assert!(state.active_async_tool_requests.is_empty());
    assert_eq!(state.abandoned_async_tool_request_count(), 1);
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .map(|request| request.tool.as_str()),
        Some("background_shell_start")
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .map(|request| request.worker_thread_name.as_str()),
        Some("codexw-async-tool-worker-15")
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .and_then(|request| request.source_call_id.as_deref()),
        Some("call-15")
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .and_then(|request| request.target_background_shell_reference.as_deref()),
        Some("dev.api")
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .and_then(|request| request.target_background_shell_job_id.as_deref()),
        Some("bg-1")
    );
    assert!(!state.async_tool_backpressure_active());
}

#[test]
fn async_tool_worker_statuses_expose_running_and_abandoned_workers() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request_with_timeout_and_worker(
        crate::rpc::RequestId::Integer(7),
        "background_shell_start".to_string(),
        "running".to_string(),
        std::time::Duration::from_secs(30),
        "codexw-bgtool-background_shell_start-7".to_string(),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(7))
    {
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(20);
    }
    state.record_async_tool_request_with_timeout_and_worker(
        crate::rpc::RequestId::Integer(8),
        "background_shell_start".to_string(),
        "abandoned".to_string(),
        std::time::Duration::from_secs(5),
        "codexw-bgtool-background_shell_start-8".to_string(),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(8))
    {
        activity.source_call_id = Some("call-8".to_string());
        activity.target_background_shell_reference = Some("dev.api".to_string());
        activity.target_background_shell_job_id = Some("bg-1".to_string());
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(80);
    }
    let _expired = state.expire_timed_out_async_tool_requests();

    let workers = state.async_tool_worker_statuses();

    assert_eq!(workers.len(), 2);
    assert_eq!(workers[0].request_id, "7");
    assert_eq!(workers[0].lifecycle_state.label(), "running");
    assert_eq!(
        workers[0].worker_thread_name,
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        workers[0]
            .supervision_classification
            .map(|classification| classification.label()),
        Some("tool_slow")
    );
    assert_eq!(
        workers[0]
            .observation_state
            .map(|observation_state| observation_state.label()),
        Some("no_job_or_output_observed_yet")
    );
    assert_eq!(
        workers[0]
            .output_state
            .map(|output_state| output_state.label()),
        Some("no_output_observed_yet")
    );
    assert_eq!(workers[0].owner_kind.label(), "wrapper_background_shell");
    assert!(workers[0].source_call_id.is_none());
    assert!(workers[0].next_health_check_in.is_some());
    assert_eq!(workers[1].request_id, "8");
    assert_eq!(
        workers[1].lifecycle_state.label(),
        "abandoned_after_timeout"
    );
    assert_eq!(
        workers[1].worker_thread_name,
        "codexw-bgtool-background_shell_start-8"
    );
    assert_eq!(workers[1].supervision_classification, None);
    assert_eq!(
        workers[1]
            .observation_state
            .map(|observation_state| observation_state.label()),
        Some("no_job_or_output_observed_yet")
    );
    assert_eq!(
        workers[1]
            .output_state
            .map(|output_state| output_state.label()),
        Some("no_output_observed_yet")
    );
    assert_eq!(workers[1].source_call_id.as_deref(), Some("call-8"));
    assert_eq!(
        workers[1].target_background_shell_reference.as_deref(),
        Some("dev.api")
    );
    assert_eq!(
        workers[1].target_background_shell_job_id.as_deref(),
        Some("bg-1")
    );
    assert!(workers[1].observed_background_shell_job.is_none());
    assert_eq!(workers[1].next_health_check_in, None);
}

#[test]
fn abandoned_async_backlog_becomes_saturated_at_threshold() {
    let mut state = crate::state::AppState::new(true, false);
    for id in 1..=crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS {
        state.record_async_tool_request_with_timeout(
            crate::rpc::RequestId::Integer(id as i64),
            "background_shell_start".to_string(),
            format!("summary-{id}"),
            std::time::Duration::from_secs(1),
        );
        if let Some(activity) = state
            .active_async_tool_requests
            .get_mut(&crate::rpc::RequestId::Integer(id as i64))
        {
            activity.started_at = std::time::Instant::now()
                - std::time::Duration::from_secs(if id == 1 { 120 } else { 80 });
        }
    }

    let expired = state.expire_timed_out_async_tool_requests();

    assert_eq!(
        expired.len(),
        crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS
    );
    assert!(state.async_tool_backpressure_active());
    assert_eq!(
        state.abandoned_async_tool_request_count(),
        crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS
    );
    assert_eq!(
        state
            .oldest_abandoned_async_tool_request()
            .map(|request| request.summary.as_str()),
        Some("summary-1")
    );
}
