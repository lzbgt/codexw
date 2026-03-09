use super::super::super::super::super::*;

#[test]
fn background_shell_list_services_can_filter_by_capability() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "label": "api svc",
                "capabilities": ["api.http"]
            }
        }),
        "/tmp",
        &manager,
    );
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "label": "frontend svc",
                "capabilities": ["frontend.dev"]
            }
        }),
        "/tmp",
        &manager,
    );

    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "capability": "@api.http"
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(result["success"], true);
    let text = result["contentItems"][0]["text"]
        .as_str()
        .expect("service list text");
    assert!(text.contains("api svc"));
    assert!(text.contains("api.http"));
    assert!(!text.contains("frontend svc"));
    let _ = manager.terminate_all_running();
}

#[test]
fn background_shell_list_services_rejects_invalid_capability() {
    let manager = BackgroundShellManager::default();
    let result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "capability": "@bad!"
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
            .contains("background shell capability")
    );
}

#[test]
fn background_shell_list_services_can_filter_service_states() {
    let manager = BackgroundShellManager::default();
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) {
                    "ping -n 2 127.0.0.1 >NUL && echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "sleep 0.15; printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "label": "booting svc",
                "capabilities": ["svc.booting"],
                "readyPattern": "READY"
            }
        }),
        "/tmp",
        &manager,
    );
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": if cfg!(windows) {
                    "echo READY && ping -n 2 127.0.0.1 >NUL"
                } else {
                    "printf 'READY\\n'; sleep 0.3"
                },
                "intent": "service",
                "label": "ready svc",
                "capabilities": ["svc.ready"],
                "readyPattern": "READY"
            }
        }),
        "/tmp",
        &manager,
    );
    execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_start",
            "arguments": {
                "command": "sleep 0.4",
                "intent": "service",
                "label": "untracked svc",
                "capabilities": ["svc.untracked"]
            }
        }),
        "/tmp",
        &manager,
    );

    let wait_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_wait_ready",
            "arguments": {
                "jobId": "bg-2",
                "timeoutMs": 2_000
            }
        }),
        "/tmp",
        &manager,
    );
    assert_eq!(wait_result["success"], true);

    let ready_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "status": "ready"
            }
        }),
        "/tmp",
        &manager,
    );
    assert_eq!(ready_result["success"], true);
    let ready_text = ready_result["contentItems"][0]["text"]
        .as_str()
        .expect("ready text");
    assert!(ready_text.contains("ready svc"));
    assert!(!ready_text.contains("booting svc"));

    let booting_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "status": "booting"
            }
        }),
        "/tmp",
        &manager,
    );
    assert_eq!(booting_result["success"], true);
    let booting_text = booting_result["contentItems"][0]["text"]
        .as_str()
        .expect("booting text");
    assert!(booting_text.contains("booting svc"));
    assert!(!booting_text.contains("ready svc"));

    let untracked_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_list_services",
            "arguments": {
                "status": "untracked"
            }
        }),
        "/tmp",
        &manager,
    );
    assert_eq!(untracked_result["success"], true);
    let untracked_text = untracked_result["contentItems"][0]["text"]
        .as_str()
        .expect("untracked text");
    assert!(untracked_text.contains("untracked svc"));
    assert!(!untracked_text.contains("ready svc"));
    let _ = manager.terminate_all_running();
}
