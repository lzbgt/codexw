use super::*;

#[test]
fn publish_snapshot_change_events_emits_status_update_when_supervision_changes() {
    let snapshot = sample_snapshot();
    let previous = snapshot.read().expect("snapshot").clone();
    let mut current = previous.clone();
    current.async_tool_supervision =
        Some(crate::local_api::snapshot::LocalApiAsyncToolSupervision {
            classification: "tool_wedged".to_string(),
            recommended_action: "interrupt_or_exit_resume".to_string(),
            recovery_policy: crate::local_api::snapshot::LocalApiRecoveryPolicy {
                kind: "operator_interrupt_or_exit_resume".to_string(),
                automation_ready: false,
            },
            recovery_options: vec![
                crate::local_api::snapshot::LocalApiRecoveryOption {
                    kind: "interrupt_turn".to_string(),
                    label: "Interrupt the active turn".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test/turn/interrupt".to_string()),
                },
                crate::local_api::snapshot::LocalApiRecoveryOption {
                    kind: "exit_and_resume".to_string(),
                    label: "Exit and resume the thread in a newer client".to_string(),
                    automation_ready: false,
                    cli_command: Some("codexw --cwd /tmp/repo resume thread_123".to_string()),
                    local_api_method: None,
                    local_api_path: None,
                },
            ],
            request_id: "7".to_string(),
            thread_name: "codexw-bgtool-background_shell_start-7".to_string(),
            owner: "wrapper_background_shell".to_string(),
            source_call_id: Some("call_1".to_string()),
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-1".to_string()),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            observation_state: "wrapper_background_shell_terminal_without_tool_response"
                .to_string(),
            output_state: "stale_output_observed".to_string(),
            observed_background_shell_job: Some(
                crate::local_api::snapshot::LocalApiObservedBackgroundShellJob {
                    job_id: "bg-1".to_string(),
                    status: "failed".to_string(),
                    command: "npm run dev".to_string(),
                    total_lines: 3,
                    last_output_age_seconds: Some(75),
                    recent_lines: vec!["boom".to_string()],
                },
            ),
            next_check_in_seconds: 30,
            elapsed_seconds: 75,
            active_request_count: 1,
        });
    current.async_tool_backpressure =
        Some(crate::local_api::snapshot::LocalApiAsyncToolBackpressure {
            abandoned_request_count: crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS,
            saturation_threshold: crate::state::MAX_ABANDONED_ASYNC_TOOL_REQUESTS,
            saturated: true,
            recommended_action: "interrupt_or_exit_resume".to_string(),
            recovery_policy: crate::local_api::snapshot::LocalApiRecoveryPolicy {
                kind: "operator_interrupt_or_exit_resume".to_string(),
                automation_ready: false,
            },
            recovery_options: vec![
                crate::local_api::snapshot::LocalApiRecoveryOption {
                    kind: "observe_status".to_string(),
                    label: "Observe current session status".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("GET".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test".to_string()),
                },
                crate::local_api::snapshot::LocalApiRecoveryOption {
                    kind: "interrupt_turn".to_string(),
                    label: "Interrupt the active turn".to_string(),
                    automation_ready: false,
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: Some("/api/v1/session/sess_test/turn/interrupt".to_string()),
                },
                crate::local_api::snapshot::LocalApiRecoveryOption {
                    kind: "exit_and_resume".to_string(),
                    label: "Exit and resume the thread in a newer client".to_string(),
                    automation_ready: false,
                    cli_command: Some("codexw --cwd /tmp/repo resume thread_123".to_string()),
                    local_api_method: None,
                    local_api_path: None,
                },
            ],
            oldest_request_id: "8".to_string(),
            oldest_thread_name: "codexw-bgtool-background_shell_start-8".to_string(),
            oldest_tool: "background_shell_start".to_string(),
            oldest_summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            oldest_source_call_id: Some("call_2".to_string()),
            oldest_target_background_shell_reference: Some("dev.api".to_string()),
            oldest_target_background_shell_job_id: Some("bg-1".to_string()),
            oldest_observation_state: "wrapper_background_shell_terminal_without_tool_response"
                .to_string(),
            oldest_output_state: "stale_output_observed".to_string(),
            oldest_observed_background_shell_job: Some(
                crate::local_api::snapshot::LocalApiObservedBackgroundShellJob {
                    job_id: "bg-1".to_string(),
                    status: "failed".to_string(),
                    command: "npm run dev".to_string(),
                    total_lines: 3,
                    last_output_age_seconds: Some(75),
                    recent_lines: vec!["boom".to_string()],
                },
            ),
            oldest_elapsed_before_timeout_seconds: 75,
            oldest_hard_timeout_seconds: 30,
            oldest_elapsed_seconds: 30,
        });
    current.async_tool_workers = vec![
        crate::local_api::snapshot::LocalApiAsyncToolWorker {
            request_id: "7".to_string(),
            lifecycle_state: "running".to_string(),
            thread_name: "codexw-bgtool-background_shell_start-7".to_string(),
            owner: "wrapper_background_shell".to_string(),
            source_call_id: Some("call_1".to_string()),
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-1".to_string()),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            observation_state: Some(
                "wrapper_background_shell_terminal_without_tool_response".to_string(),
            ),
            output_state: Some("stale_output_observed".to_string()),
            observed_background_shell_job: Some(
                crate::local_api::snapshot::LocalApiObservedBackgroundShellJob {
                    job_id: "bg-1".to_string(),
                    status: "failed".to_string(),
                    command: "npm run dev".to_string(),
                    total_lines: 3,
                    last_output_age_seconds: Some(75),
                    recent_lines: vec!["boom".to_string()],
                },
            ),
            next_check_in_seconds: Some(30),
            runtime_elapsed_seconds: 75,
            state_elapsed_seconds: 75,
            hard_timeout_seconds: 30,
            supervision_classification: Some("tool_wedged".to_string()),
        },
        crate::local_api::snapshot::LocalApiAsyncToolWorker {
            request_id: "8".to_string(),
            lifecycle_state: "abandoned_after_timeout".to_string(),
            thread_name: "codexw-bgtool-background_shell_start-8".to_string(),
            owner: "wrapper_background_shell".to_string(),
            source_call_id: Some("call_2".to_string()),
            target_background_shell_reference: Some("dev.api".to_string()),
            target_background_shell_job_id: Some("bg-1".to_string()),
            tool: "background_shell_start".to_string(),
            summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
            observation_state: Some(
                "wrapper_background_shell_terminal_without_tool_response".to_string(),
            ),
            output_state: Some("stale_output_observed".to_string()),
            observed_background_shell_job: Some(
                crate::local_api::snapshot::LocalApiObservedBackgroundShellJob {
                    job_id: "bg-1".to_string(),
                    status: "failed".to_string(),
                    command: "npm run dev".to_string(),
                    total_lines: 3,
                    last_output_age_seconds: Some(75),
                    recent_lines: vec!["boom".to_string()],
                },
            ),
            next_check_in_seconds: None,
            runtime_elapsed_seconds: 30,
            state_elapsed_seconds: 30,
            hard_timeout_seconds: 30,
            supervision_classification: None,
        },
    ];
    current.supervision_notice = Some(crate::local_api::snapshot::LocalApiSupervisionNotice {
        classification: "tool_wedged".to_string(),
        recommended_action: "interrupt_or_exit_resume".to_string(),
        recovery_policy: crate::local_api::snapshot::LocalApiRecoveryPolicy {
            kind: "operator_interrupt_or_exit_resume".to_string(),
            automation_ready: false,
        },
        recovery_options: vec![
            crate::local_api::snapshot::LocalApiRecoveryOption {
                kind: "interrupt_turn".to_string(),
                label: "Interrupt the active turn".to_string(),
                automation_ready: false,
                cli_command: None,
                local_api_method: Some("POST".to_string()),
                local_api_path: Some("/api/v1/session/sess_test/turn/interrupt".to_string()),
            },
            crate::local_api::snapshot::LocalApiRecoveryOption {
                kind: "exit_and_resume".to_string(),
                label: "Exit and resume the thread in a newer client".to_string(),
                automation_ready: false,
                cli_command: Some("codexw --cwd /tmp/repo resume thread_123".to_string()),
                local_api_method: None,
                local_api_path: None,
            },
        ],
        request_id: "7".to_string(),
        thread_name: "codexw-bgtool-background_shell_start-7".to_string(),
        owner: "wrapper_background_shell".to_string(),
        source_call_id: Some("call_1".to_string()),
        target_background_shell_reference: Some("dev.api".to_string()),
        target_background_shell_job_id: Some("bg-1".to_string()),
        tool: "background_shell_start".to_string(),
        summary: "arguments= command=sleep 5 tool=background_shell_start".to_string(),
        observation_state: "wrapper_background_shell_terminal_without_tool_response".to_string(),
        output_state: "stale_output_observed".to_string(),
        observed_background_shell_job: Some(
            crate::local_api::snapshot::LocalApiObservedBackgroundShellJob {
                job_id: "bg-1".to_string(),
                status: "failed".to_string(),
                command: "npm run dev".to_string(),
                total_lines: 3,
                last_output_age_seconds: Some(75),
                recent_lines: vec!["boom".to_string()],
            },
        ),
    });
    let log = new_event_log();

    publish_snapshot_change_events(&log, Some(&previous), &current);

    let events = events_since(&log, "sess_test", None);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event, "session.updated");
    assert_eq!(
        events[0].data["session"]["async_tool_supervision"]["classification"],
        "tool_wedged"
    );
    assert_eq!(
        events[0].data["session"]["async_tool_supervision"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["source_call_id"],
        "call_1"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["recovery_policy"]["kind"],
        "operator_interrupt_or_exit_resume"
    );
    assert_eq!(
        events[0].data["session"]["supervision_notice"]["recovery_options"][1]["kind"],
        "exit_and_resume"
    );
    assert_eq!(events[1].event, "status.updated");
    assert_eq!(
        events[1].data["async_tool_supervision"]["classification"],
        "tool_wedged"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["recommended_action"],
        "interrupt_or_exit_resume"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(events[1].data["async_tool_supervision"]["request_id"], "7");
    assert_eq!(
        events[1].data["async_tool_supervision"]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["observation_state"],
        "wrapper_background_shell_terminal_without_tool_response"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["output_state"],
        "stale_output_observed"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["observed_background_shell_job"]["status"],
        "failed"
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["observed_background_shell_job"]["last_output_age_seconds"],
        75
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["next_check_in_seconds"],
        30
    );
    assert_eq!(
        events[1].data["async_tool_workers"][0]["supervision_classification"],
        "tool_wedged"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][0]["observation_state"],
        "wrapper_background_shell_terminal_without_tool_response"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][0]["output_state"],
        "stale_output_observed"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][0]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][0]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][0]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][1]["lifecycle_state"],
        "abandoned_after_timeout"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][1]["observation_state"],
        "wrapper_background_shell_terminal_without_tool_response"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][1]["output_state"],
        "stale_output_observed"
    );
    assert_eq!(
        events[1].data["async_tool_workers"][1]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(events[1].data["async_tool_backpressure"]["saturated"], true);
    assert_eq!(
        events[1].data["async_tool_backpressure"]["oldest_request_id"],
        "8"
    );
    assert_eq!(
        events[1].data["async_tool_backpressure"]["oldest_thread_name"],
        "codexw-bgtool-background_shell_start-8"
    );
    assert_eq!(
        events[1].data["async_tool_backpressure"]["oldest_observation_state"],
        "wrapper_background_shell_terminal_without_tool_response"
    );
    assert_eq!(
        events[1].data["async_tool_backpressure"]["oldest_output_state"],
        "stale_output_observed"
    );
    assert_eq!(
        events[1].data["async_tool_backpressure"]["oldest_observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["classification"],
        "tool_wedged"
    );
    assert_eq!(events[1].data["supervision_notice"]["request_id"], "7");
    assert_eq!(
        events[1].data["supervision_notice"]["thread_name"],
        "codexw-bgtool-background_shell_start-7"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["owner"],
        "wrapper_background_shell"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["source_call_id"],
        "call_1"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["target_background_shell_reference"],
        "dev.api"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["target_background_shell_job_id"],
        "bg-1"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["observation_state"],
        "wrapper_background_shell_terminal_without_tool_response"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["output_state"],
        "stale_output_observed"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["observed_background_shell_job"]["job_id"],
        "bg-1"
    );
    assert_eq!(
        events[1].data["supervision_notice"]["recovery_policy"]["automation_ready"],
        false
    );
    assert_eq!(
        events[1].data["async_tool_supervision"]["recovery_options"][0]["local_api_path"],
        "/api/v1/session/sess_test/turn/interrupt"
    );
}
