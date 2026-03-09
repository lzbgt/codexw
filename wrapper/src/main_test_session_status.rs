use crate::Cli;
use crate::background_terminals::background_terminal_count;
use crate::background_terminals::render_background_terminals;
use crate::dispatch_command_session_ps::handle_ps_command;
use crate::dispatch_command_session_ps::parse_clean_selection;
use crate::dispatch_command_session_ps::parse_clean_target;
use crate::dispatch_command_session_ps::parse_ps_capability_issue_filter;
use crate::dispatch_command_session_ps::parse_ps_dependency_filter;
use crate::dispatch_command_session_ps::parse_ps_dependency_selector;
use crate::dispatch_command_session_ps::parse_ps_filter;
use crate::dispatch_command_session_ps::parse_ps_focus_capability;
use crate::dispatch_command_session_ps::parse_ps_service_issue_filter;
use crate::dispatch_command_session_ps::parse_ps_service_selector;
use crate::events::handle_realtime_notification;
use crate::notification_item_buffers::handle_buffer_update;
use crate::notification_item_completion::render_item_completed;
use crate::notification_item_status::handle_status_update;
use crate::orchestration_registry::LiveAgentTaskSummary;
use crate::orchestration_view::CachedAgentThreadSummary;
use crate::orchestration_view::WorkerFilter;
use crate::output::Output;
use crate::prompt_state::render_prompt_status;
use crate::session_prompt_status_active::spinner_frame;
use crate::session_snapshot_overview::render_status_overview;
use crate::session_snapshot_runtime::render_status_runtime;
use crate::transcript_status_summary::summarize_thread_status_for_display;
pub(crate) use serde_json::json;
pub(crate) use std::process::ChildStdin;
pub(crate) use std::process::Command;
pub(crate) use std::time::Duration;

#[path = "main_test_session_status/prompt.rs"]
mod prompt;
#[path = "main_test_session_status_ps_orchestration.rs"]
mod ps_orchestration;
#[path = "main_test_session_status_ps_recipes.rs"]
mod ps_recipes;
#[path = "main_test_session_status_ps_services.rs"]
mod ps_services;
#[path = "main_test_session_status/runtime.rs"]
mod runtime;
#[path = "main_test_session_status/state.rs"]
mod state;

fn test_cli() -> Cli {
    crate::runtime_process::normalize_cli(Cli {
        codex_bin: "codex".to_string(),
        config_overrides: Vec::new(),
        enable_features: Vec::new(),
        disable_features: Vec::new(),
        resume: None,
        resume_picker: false,
        cwd: None,
        model: None,
        model_provider: None,
        auto_continue: true,
        verbose_events: false,
        verbose_thinking: true,
        raw_json: false,
        no_experimental_api: false,
        yolo: false,
        local_api: false,
        local_api_bind: "127.0.0.1:0".to_string(),
        local_api_token: None,
        prompt: Vec::new(),
    })
}

fn spawn_sink_stdin() -> ChildStdin {
    #[cfg(unix)]
    let mut child = Command::new("cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawn sink");
    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args(["/C", "more >NUL"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawn sink");
    child.stdin.take().expect("child stdin")
}
