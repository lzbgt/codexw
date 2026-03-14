use crate::app::build_resume_command;
use crate::app::current_program_name;
use crate::state::AppState;
use crate::state::AsyncToolSupervisionClass;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SupervisionRecoveryOption {
    pub(crate) kind: &'static str,
    pub(crate) label: &'static str,
    pub(crate) automation_ready: bool,
    pub(crate) terminal_command: Option<String>,
    pub(crate) cli_command: Option<String>,
    pub(crate) local_api_method: Option<String>,
    pub(crate) local_api_path: Option<String>,
}

pub(crate) fn supervision_recovery_options(
    state: &AppState,
    session_id: Option<&str>,
    cwd: &str,
    classification: AsyncToolSupervisionClass,
) -> Vec<SupervisionRecoveryOption> {
    let mut options = Vec::new();
    match classification {
        AsyncToolSupervisionClass::ToolSlow => {
            options.push(SupervisionRecoveryOption {
                kind: "observe_status",
                label: "Observe current session status",
                automation_ready: false,
                terminal_command: Some(":status".to_string()),
                cli_command: None,
                local_api_method: Some("GET".to_string()),
                local_api_path: session_id.map(|id| format!("/api/v1/session/{id}")),
            });
            if state.turn_running || state.active_turn_id.is_some() {
                options.push(SupervisionRecoveryOption {
                    kind: "interrupt_turn",
                    label: "Interrupt the active turn",
                    automation_ready: false,
                    terminal_command: Some(":interrupt".to_string()),
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: session_id
                        .map(|id| format!("/api/v1/session/{id}/turn/interrupt")),
                });
            }
        }
        AsyncToolSupervisionClass::ToolWedged => {
            if state.turn_running || state.active_turn_id.is_some() {
                options.push(SupervisionRecoveryOption {
                    kind: "interrupt_turn",
                    label: "Interrupt the active turn",
                    automation_ready: false,
                    terminal_command: Some(":interrupt".to_string()),
                    cli_command: None,
                    local_api_method: Some("POST".to_string()),
                    local_api_path: session_id
                        .map(|id| format!("/api/v1/session/{id}/turn/interrupt")),
                });
            }
            if let Some(thread_id) = state.thread_id.as_deref() {
                options.push(SupervisionRecoveryOption {
                    kind: "exit_and_resume",
                    label: "Exit and resume the thread in a newer client",
                    automation_ready: false,
                    terminal_command: None,
                    cli_command: Some(build_resume_command(
                        &current_program_name(),
                        cwd,
                        thread_id,
                    )),
                    local_api_method: None,
                    local_api_path: None,
                });
            }
        }
    }
    options
}

pub(crate) fn async_backpressure_recovery_options(
    state: &AppState,
    session_id: Option<&str>,
    cwd: &str,
) -> Vec<SupervisionRecoveryOption> {
    let mut options = vec![SupervisionRecoveryOption {
        kind: "observe_status",
        label: "Observe current session status",
        automation_ready: false,
        terminal_command: Some(":status".to_string()),
        cli_command: None,
        local_api_method: Some("GET".to_string()),
        local_api_path: session_id.map(|id| format!("/api/v1/session/{id}")),
    }];
    if state.turn_running || state.active_turn_id.is_some() {
        options.push(SupervisionRecoveryOption {
            kind: "interrupt_turn",
            label: "Interrupt the active turn",
            automation_ready: false,
            terminal_command: Some(":interrupt".to_string()),
            cli_command: None,
            local_api_method: Some("POST".to_string()),
            local_api_path: session_id.map(|id| format!("/api/v1/session/{id}/turn/interrupt")),
        });
    }
    if let Some(thread_id) = state.thread_id.as_deref() {
        options.push(SupervisionRecoveryOption {
            kind: "exit_and_resume",
            label: "Exit and resume the thread in a newer client",
            automation_ready: false,
            terminal_command: None,
            cli_command: Some(build_resume_command(
                &current_program_name(),
                cwd,
                thread_id,
            )),
            local_api_method: None,
            local_api_path: None,
        });
    }
    options
}
