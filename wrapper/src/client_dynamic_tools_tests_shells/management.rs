use super::AppState;
use super::BackgroundShellManager;
use super::execute_dynamic_tool_call;
use super::execute_dynamic_tool_call_with_state;
use super::json;

#[path = "management/service_controls.rs"]
mod service_controls;

#[test]
fn background_shell_clean_can_resolve_ambiguous_service_capability_conflicts() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api a",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start first provider");
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api b",
                "capabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start second provider");
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "service",
                "label": "db",
                "capabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start unrelated provider");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "background_shell_clean",
            "arguments": {
                "scope": "services",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("cleanup text");
    assert!(text.contains("Terminated 2"));
    assert!(text.contains("@api.http"));

    let remaining = state
        .orchestration
        .background_shells
        .render_service_shells_for_ps_filtered(None, None)
        .expect("render remaining services")
        .join("\n");
    assert!(!remaining.contains("api a"));
    assert!(!remaining.contains("api b"));
    assert!(remaining.contains("db"));

    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn background_shell_clean_can_target_blockers_by_capability() {
    let state = AppState::new(true, false);
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "api blocker",
                "dependsOnCapabilities": ["api.http"]
            }),
            "/tmp",
        )
        .expect("start api blocker");
    state
        .orchestration
        .background_shells
        .start_from_tool(
            &json!({
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "db blocker",
                "dependsOnCapabilities": ["db.redis"]
            }),
            "/tmp",
        )
        .expect("start db blocker");

    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "background_shell_clean",
            "arguments": {
                "scope": "blockers",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("cleanup text");
    assert!(text.contains("Terminated 1"));
    assert!(text.contains("@api.http"));

    let remaining = state
        .orchestration
        .background_shells
        .capability_dependency_summaries()
        .into_iter()
        .map(|summary| format!("{} -> {}", summary.job_id, summary.capability))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!remaining.contains("api.http"));
    assert!(remaining.contains("db.redis"));
    let _ = state
        .orchestration
        .background_shells
        .terminate_all_running();
}

#[test]
fn background_shell_clean_rejects_capability_outside_service_scope() {
    let state = AppState::new(true, false);
    let result = execute_dynamic_tool_call_with_state(
        &json!({
            "tool": "background_shell_clean",
            "arguments": {
                "scope": "shells",
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &state,
    );
    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error")
            .contains("only valid with `scope=blockers` or `scope=services`")
    );
}

#[test]
fn background_shell_start_preserves_request_origin_metadata() {
    let manager = BackgroundShellManager::default();
    let result = execute_dynamic_tool_call(
        &json!({
            "threadId": "thread-agent-1",
            "callId": "call-55",
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "prerequisite",
                "label": "repo build",
                "dependsOnCapabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let snapshots = manager.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(
        snapshots[0].origin.source_thread_id.as_deref(),
        Some("thread-agent-1")
    );
    assert_eq!(
        snapshots[0].origin.source_call_id.as_deref(),
        Some("call-55")
    );
    assert_eq!(snapshots[0].intent.as_str(), "prerequisite");
    assert_eq!(snapshots[0].label.as_deref(), Some("repo build"));
    assert_eq!(
        snapshots[0].dependency_capabilities,
        vec!["api.http".to_string()]
    );
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_send_writes_to_alias_target() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) { "more" } else { "cat" },
                "intent": "service",
                "label": "echo shell"
            }
        }),
        "/tmp",
        &manager,
    );
    manager.set_job_alias("bg-1", "dev.api").expect("set alias");

    let send_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_send",
            "arguments": {
                "jobId": "dev.api",
                "text": "ping from tool"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(send_result["success"], true);
    let mut rendered = String::new();
    for _ in 0..40 {
        let poll_result = execute_dynamic_tool_call(
            &json!({
                "tool": "background_shell_poll",
                "arguments": {
                    "jobId": "dev.api"
                }
            }),
            "/tmp",
            &manager,
        );
        rendered = poll_result["contentItems"][0]["text"]
            .as_str()
            .expect("poll text")
            .to_string();
        if rendered.contains("ping from tool") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    assert!(rendered.contains("ping from tool"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_set_alias_can_assign_and_clear_alias() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "observation"
            }
        }),
        "/tmp",
        &manager,
    );

    let assign_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_set_alias",
            "arguments": {
                "jobId": "bg-1",
                "alias": "dev.api"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(assign_result["success"], true);
    assert!(
        assign_result["contentItems"][0]["text"]
            .as_str()
            .expect("assign text")
            .contains("Aliased background shell job bg-1 as dev.api")
    );
    assert_eq!(
        manager
            .resolve_job_reference("dev.api")
            .expect("resolve alias"),
        "bg-1"
    );

    let clear_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_set_alias",
            "arguments": {
                "jobId": "dev.api",
                "alias": null
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(clear_result["success"], true);
    assert!(
        clear_result["contentItems"][0]["text"]
            .as_str()
            .expect("clear text")
            .contains("Cleared alias for background shell job bg-1")
    );
    assert!(manager.resolve_job_reference("dev.api").is_err());
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_set_alias_reports_validation_errors() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "observation"
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_set_alias",
            "arguments": {
                "jobId": "bg-1",
                "alias": 123
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], false);
    assert!(
        result["contentItems"][0]["text"]
            .as_str()
            .expect("error text")
            .contains("`alias` must be a string or null")
    );
    let _ = manager.terminate_all_running();
}
