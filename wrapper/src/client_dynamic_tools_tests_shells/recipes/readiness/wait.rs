use super::super::super::BackgroundShellManager;
use super::super::super::execute_dynamic_tool_call;
use super::super::super::json;

#[test]
fn background_shell_wait_ready_reports_ready_services() {
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
                "readyPattern": "READY"
            }
        }),
        "/tmp",
        &manager,
    );

    let wait_result = execute_dynamic_tool_call(
        &json!({
            "tool": "background_shell_wait_ready",
            "arguments": {
                "jobId": "bg-1",
                "timeoutMs": 2_000
            }
        }),
        "/tmp",
        &manager,
    );

    assert_eq!(wait_result["success"], true);
    let rendered = wait_result["contentItems"][0]["text"]
        .as_str()
        .expect("wait text");
    assert!(rendered.contains("Ready pattern: READY"));
    assert!(rendered.contains("ready"));
    let _ = manager.terminate_all_running();
}
