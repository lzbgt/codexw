#[test]
fn async_tool_supervision_classifies_slow_and_wedged_elapsed_time() {
    let mut state = crate::state::AppState::new(true, false);
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "slow".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(20),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.label()),
        Some("tool_slow")
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.recommended_action()),
        Some("observe_or_interrupt")
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.recovery_policy_kind().label()),
        Some("warn_only")
    );

    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "wedged".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(65),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.label()),
        Some("tool_wedged")
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.recommended_action()),
        Some("interrupt_or_exit_resume")
    );
    assert_eq!(
        state
            .oldest_async_tool_supervision_class()
            .map(|class| class.recovery_policy_kind().label()),
        Some("operator_interrupt_or_exit_resume")
    );
}

#[test]
fn async_tool_supervision_notice_tracks_raise_escalation_and_clear() {
    let mut state = crate::state::AppState::new(true, false);
    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "slow".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(20),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );

    let raised = state.refresh_async_tool_supervision_notice();
    assert!(matches!(
        raised,
        Some(crate::state::SupervisionNoticeTransition::Raised(_))
    ));
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.classification.label()),
        Some("tool_slow")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.recovery_policy_kind().label()),
        Some("warn_only")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.request_id.as_str()),
        Some("10")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.worker_thread_name.as_str()),
        Some("codexw-bgtool-background_shell_start-10")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.owner_kind.label()),
        Some("wrapper_background_shell")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.observation_state.label()),
        Some("no_job_or_output_observed_yet")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.output_state.label()),
        Some("no_output_observed_yet")
    );

    state.active_async_tool_requests.insert(
        crate::rpc::RequestId::Integer(10),
        crate::state::AsyncToolActivity {
            tool: "background_shell_start".to_string(),
            summary: "wedged".to_string(),
            owner_kind: crate::state::AsyncToolOwnerKind::WrapperBackgroundShell,
            source_call_id: None,
            target_background_shell_reference: None,
            target_background_shell_job_id: None,
            worker_thread_name: "codexw-bgtool-background_shell_start-10".to_string(),
            started_at: std::time::Instant::now() - std::time::Duration::from_secs(75),
            hard_timeout: crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            next_health_check_after: crate::state::AsyncToolActivity::initial_health_check_interval(
                crate::state::DEFAULT_ASYNC_TOOL_REQUEST_TIMEOUT,
            ),
        },
    );
    let escalated = state.refresh_async_tool_supervision_notice();
    assert!(matches!(
        escalated,
        Some(crate::state::SupervisionNoticeTransition::Raised(_))
    ));
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.classification.label()),
        Some("tool_wedged")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.recovery_policy_kind().label()),
        Some("operator_interrupt_or_exit_resume")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.request_id.as_str()),
        Some("10")
    );
    assert_eq!(
        state
            .active_supervision_notice
            .as_ref()
            .map(|notice| notice.worker_thread_name.as_str()),
        Some("codexw-bgtool-background_shell_start-10")
    );

    state.active_async_tool_requests.clear();
    let cleared = state.refresh_async_tool_supervision_notice();
    assert_eq!(
        cleared,
        Some(crate::state::SupervisionNoticeTransition::Cleared)
    );
    assert!(state.active_supervision_notice.is_none());
}

#[test]
fn async_tool_health_checks_are_scheduled_by_orchestrator_policy() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request_with_timeout_and_worker(
        crate::rpc::RequestId::Integer(77),
        "background_shell_start".to_string(),
        "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        std::time::Duration::from_secs(15),
        "codexw-bgtool-background_shell_start-77".to_string(),
    );
    let activity = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(77))
        .expect("activity");
    assert_eq!(activity.next_health_check_after.as_secs(), 5);

    activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(6);
    let checks = state.collect_due_async_tool_health_checks();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].request_id, "77");
    assert_eq!(checks[0].tool, "background_shell_start");
    assert!(checks[0].summary.contains("command=sleep 5"));
    assert_eq!(checks[0].supervision_classification, None);
    assert_eq!(
        checks[0].observation_state.label(),
        "no_job_or_output_observed_yet"
    );

    let activity = state
        .active_async_tool_requests
        .get(&crate::rpc::RequestId::Integer(77))
        .expect("activity");
    assert_eq!(activity.next_health_check_after.as_secs(), 11);
}

#[test]
fn async_tool_health_checks_escalate_when_worker_becomes_slow() {
    let mut state = crate::state::AppState::new(true, false);
    state.record_async_tool_request_with_timeout_and_worker(
        crate::rpc::RequestId::Integer(78),
        "background_shell_wait_ready".to_string(),
        "jobId=bg-1 timeoutMs=60000".to_string(),
        std::time::Duration::from_secs(65),
        "codexw-bgtool-background_shell_wait_ready-78".to_string(),
    );
    if let Some(activity) = state
        .active_async_tool_requests
        .get_mut(&crate::rpc::RequestId::Integer(78))
    {
        activity.started_at = std::time::Instant::now() - std::time::Duration::from_secs(20);
        activity.next_health_check_after = std::time::Duration::from_secs(15);
    }

    let checks = state.collect_due_async_tool_health_checks();

    assert_eq!(checks.len(), 1);
    assert_eq!(
        checks[0]
            .supervision_classification
            .map(|classification| classification.label()),
        Some("tool_slow")
    );
    assert_eq!(
        checks[0].observation_state.label(),
        "no_job_or_output_observed_yet"
    );
    assert_eq!(
        state
            .active_async_tool_requests
            .get(&crate::rpc::RequestId::Integer(78))
            .expect("activity")
            .next_health_check_after
            .as_secs(),
        35
    );
}
