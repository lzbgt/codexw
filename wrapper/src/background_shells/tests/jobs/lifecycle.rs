use super::super::*;

#[test]
fn background_shell_job_can_start_and_poll_output() {
    let manager = BackgroundShellManager::default();
    let started = manager
        .start_from_tool(&json!({"command": "printf 'alpha\\nbeta\\n'"}), "/tmp")
        .expect("start background shell");
    assert!(started.contains("Started background shell job bg-1"));

    let mut rendered = String::new();
    for _ in 0..20 {
        rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll background shell");
        if rendered.contains("alpha") && rendered.contains("beta") {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    assert!(rendered.contains("Job: bg-1"));
    assert!(rendered.contains("alpha"));
    assert!(rendered.contains("beta"));
}

#[test]
fn background_shell_job_accepts_stdin_and_emits_output() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(&json!({"command": interactive_echo_command()}), "/tmp")
        .expect("start interactive background shell");

    manager
        .send_input_for_operator("bg-1", "hello from stdin", true)
        .expect("send stdin");

    let mut rendered = String::new();
    for _ in 0..40 {
        rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll background shell");
        if rendered.contains("hello from stdin") {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    assert!(rendered.contains("hello from stdin"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_job_nonzero_exit_is_reported_as_failed() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(&json!({"command": "printf 'boom\\n'; exit 2"}), "/tmp")
        .expect("start failing background shell");

    let mut rendered = String::new();
    for _ in 0..20 {
        rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll failed background shell");
        if rendered.contains("Status: failed") && rendered.contains("Exit code: 2") {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    assert!(rendered.contains("Status: failed"));
    assert!(rendered.contains("Exit code: 2"));
    assert!(rendered.contains("Terminal state: failed with exit code 2."));
    assert!(!rendered.contains("Status: terminated"));
}

#[test]
fn background_shell_poll_rejects_exhausted_cursor_on_terminal_job() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(&json!({"command": "printf 'alpha\\n'; exit 2"}), "/tmp")
        .expect("start failing background shell");

    let mut rendered = String::new();
    for _ in 0..20 {
        rendered = manager
            .poll_from_tool(&json!({"jobId": "bg-1"}))
            .expect("poll terminal background shell");
        if rendered.contains("alpha") && rendered.contains("Next afterLine: 1") {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    let err = manager
        .poll_from_tool(&json!({"jobId": "bg-1", "afterLine": 1}))
        .expect_err("exhausted terminal poll should fail");
    assert!(err.contains("terminal state (failed with exit code 2)"));
    assert!(err.contains("Stop polling this job"));
}

#[test]
fn background_shell_list_reports_running_jobs() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start background shell");
    let rendered = manager.list_from_tool();
    assert!(rendered.contains("Background shell jobs:"));
    assert!(rendered.contains("bg-1"));
    assert!(rendered.contains("running"));
    assert!(rendered.contains("intent=observation"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_origin_intent_and_label_are_preserved_in_snapshots_and_poll() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool_with_context(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "webpack dev server",
                "capabilities": ["web.dev", "frontend"],
                "protocol": "http",
                "endpoint": "http://127.0.0.1:3000",
                "attachHint": "Open the dev server in a browser",
                "recipes": [
                    {
                        "name": "health",
                        "description": "Check health",
                        "example": "curl http://127.0.0.1:3000/health",
                        "action": {
                            "type": "http",
                            "method": "GET",
                            "path": "/health"
                        }
                    }
                ]
            }),
            "/tmp",
            BackgroundShellOrigin {
                source_thread_id: Some("thread-agent-1".to_string()),
                source_call_id: Some("call-77".to_string()),
                source_tool: Some("background_shell_start".to_string()),
            },
        )
        .expect("start background shell");

    let snapshots = manager.snapshots();
    assert_eq!(
        snapshots[0].origin.source_thread_id.as_deref(),
        Some("thread-agent-1")
    );
    assert_eq!(snapshots[0].intent, BackgroundShellIntent::Service);
    assert_eq!(snapshots[0].label.as_deref(), Some("webpack dev server"));
    assert_eq!(
        snapshots[0].service_capabilities,
        vec!["frontend".to_string(), "web.dev".to_string()]
    );
    assert_eq!(snapshots[0].service_protocol.as_deref(), Some("http"));
    assert_eq!(
        snapshots[0].service_endpoint.as_deref(),
        Some("http://127.0.0.1:3000")
    );
    assert_eq!(
        snapshots[0].attach_hint.as_deref(),
        Some("Open the dev server in a browser")
    );
    let rendered = manager
        .poll_from_tool(&json!({"jobId": "bg-1"}))
        .expect("poll background shell");
    assert!(rendered.contains("Intent: service"));
    assert!(rendered.contains("Label: webpack dev server"));
    assert!(rendered.contains("Capabilities: frontend, web.dev"));
    assert!(rendered.contains("Protocol: http"));
    assert!(rendered.contains("Endpoint: http://127.0.0.1:3000"));
    assert!(rendered.contains("Attach hint: Open the dev server in a browser"));
    assert!(rendered.contains("Recipes:"));
    assert!(rendered.contains("health [http GET /health]: Check health"));
    assert!(rendered.contains("Source thread: thread-agent-1"));
    assert!(rendered.contains("Source call: call-77"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_manager_counts_running_jobs_by_intent() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite background shell");
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start service background shell");
    manager
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start observation background shell");

    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Prerequisite),
        1
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Service),
        1
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Observation),
        1
    );
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_manager_can_terminate_only_selected_intent() {
    let manager = BackgroundShellManager::default();
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "prerequisite"}),
            "/tmp",
        )
        .expect("start prerequisite background shell");
    manager
        .start_from_tool(
            &json!({"command": "sleep 0.4", "intent": "service"}),
            "/tmp",
        )
        .expect("start service background shell");
    manager
        .start_from_tool(&json!({"command": "sleep 0.4"}), "/tmp")
        .expect("start observation background shell");

    assert_eq!(
        manager.terminate_running_by_intent(BackgroundShellIntent::Service),
        1
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Service),
        0
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Prerequisite),
        1
    );
    assert_eq!(
        manager.running_count_by_intent(BackgroundShellIntent::Observation),
        1
    );
    let _ = manager.terminate_all_running();
}
